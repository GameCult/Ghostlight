//! The one consumer-neutral vocabulary an unconfined author running actors
//! needs, and nothing about turns.
//!
//! Dungeon runs one play agent under the `Play` authority. It reads the whole
//! world, decides consequences, and commits them through the kernel, but it
//! is a consumer: it cannot widen a kernel type, and it cannot build a
//! `ComponentOp`, a `Declaration`, or a `DecisionInvocation` by hand. This
//! module is the library's own renderer and its own two decoders for that
//! vocabulary, so Dungeon composes rather than reimplements.
//!
//! `table_view` is the play agent's whole-world view: the agent holds `Play`,
//! so it may see standing and secrets no subject-facing surface shows. It
//! must never reach a Persona prompt, and nothing in `PersonaLane` or the
//! Projector may call it.

use super::controllers::{
    self, ControllerError, InferenceEvent, InferenceOutput, InferencePurpose, InferenceRequest,
    RequestShape, tool_request,
};
use super::patch::{
    self, AccessKind, ComponentOp, Declaration, DependencyTarget, FactStandingRef, PATCH_TOOLS,
    PatchToolShape, PressureSource, Reach,
};
use super::{
    ActionMismatch, AffordanceSnapshot, CommandId, ControllerMode, DecisionInvocation,
    DecisionOpportunity, EntityId, KernelError, Mismatch, RefKind, Statement, SubjectId,
    SubjectKind, Target, WorldPatch, WorldSnapshot,
};
use codex_connector::{CodexInputItem, CodexToolDefinition};
use serde_json::Value;
use thiserror::Error;

/// `authoring_tools` refuses a name outside `PATCH_TOOLS` rather than
/// silently dropping it: a Dungeon-supplied list that names a stale or
/// misspelled tool must fail loudly, not hand the agent a shorter catalog
/// than it asked for.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum TableError {
    #[error("`{0}` is not an authoring tool")]
    UnknownAuthoringTool(String),
}

/// A named subset of `PATCH_TOOLS`, in the order named. The library owns
/// `PATCH_TOOLS`; which names a particular agent gets is policy, and policy
/// belongs to the caller — `PLAY_TOOLS` is Dungeon's, pinned in Cut 8, not
/// this module's.
pub fn authoring_tools(names: &[&str]) -> Result<Vec<CodexToolDefinition>, TableError> {
    let catalog = patch::patch_tools();
    names
        .iter()
        .map(|name| {
            catalog
                .iter()
                .find(|tool| tool.name == *name)
                .cloned()
                .ok_or_else(|| TableError::UnknownAuthoringTool((*name).to_owned()))
        })
        .collect()
}

/// One authoring tool call, decoded to the one-item patch it names: exactly
/// one declaration or one operation, plus any evidence it cites. The one-call
/// case of the same code `decode_authoring_calls` shares
/// (`decode_one_authoring_call`), kept as its own entry point with its own
/// unprefixed error text: `apply_tool_call`'s `Declare`/`Operate` arms and
/// the tests that check one tool's own example have no call index to name.
pub fn decode_authoring_call(name: &str, arguments: &str) -> Result<WorldPatch, String> {
    decode_one_authoring_call(name, arguments)
}

/// Where one item in a decoded batch's `patch` came from: a declaration at
/// this index in `patch.declarations`, or an operation at this index in
/// `patch.operations`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PatchSite {
    Declaration(usize),
    Operation(usize),
}

/// The result of decoding one maximal run of consecutive authoring calls:
/// the whole run's `WorldPatch`, plus which call produced each site in it.
/// Dungeon commits the run atomically under one command id; draft handles
/// declared by an earlier call in the run resolve against later calls in the
/// same run because every call's items land in the one shared `patch`.
#[derive(Debug)]
pub struct DecodedBatch {
    pub patch: WorldPatch,
    /// `site_calls[&site]` is the index into `calls` (the slice
    /// `decode_authoring_calls` was given) that produced that site. A
    /// refusal names its call the same way, through this same index.
    pub site_calls: std::collections::BTreeMap<PatchSite, usize>,
    /// `evidence_calls[position]` is the index into `calls` that produced
    /// `patch.evidence[position]`, in the same append order `patch.evidence`
    /// is built in (PA.f73: an `EmptyEvidence` mismatch's own `position` is
    /// an index into this same list, and previously had no call to map to).
    pub evidence_calls: Vec<usize>,
}

/// One run of authoring calls, decoded to one shared patch (PA.f54): each
/// call still decodes to exactly one declaration or one operation — the same
/// one-item rule `decode_authoring_call` documents — but every call's item
/// lands in the same `WorldPatch`, so a later call's reference to an earlier
/// call's draft handle resolves within the run. A decode error names the
/// call's index in `calls`, not merely its tool name, because the same name
/// can appear more than once in a run.
pub fn decode_authoring_calls(calls: &[(&str, &str)]) -> Result<DecodedBatch, String> {
    let mut patch = WorldPatch::default();
    let mut site_calls = std::collections::BTreeMap::new();
    let mut evidence_calls = Vec::new();
    for (call_index, (name, arguments)) in calls.iter().enumerate() {
        let item = decode_one_authoring_call(name, arguments)
            .map_err(|error| format!("call #{call_index} (`{name}`): {error}"))?;
        for declaration in item.declarations {
            site_calls.insert(PatchSite::Declaration(patch.declarations.len()), call_index);
            patch.declarations.push(declaration);
        }
        for operation in item.operations {
            site_calls.insert(PatchSite::Operation(patch.operations.len()), call_index);
            patch.operations.push(operation);
        }
        for evidence in item.evidence {
            evidence_calls.push(call_index);
            patch.evidence.push(evidence);
        }
    }
    Ok(DecodedBatch {
        patch,
        site_calls,
        evidence_calls,
    })
}

/// The one decode arm both `decode_authoring_call` and `decode_authoring_calls`
/// read: one authoring tool call to the one-item patch it names. Also
/// `apply_tool_call`'s own decode arm, factored out so it has one owner;
/// `apply_tool_call` calls it for every `Declare` or `Operate` shape rather
/// than carrying a second copy of the same match.
fn decode_one_authoring_call(name: &str, arguments: &str) -> Result<WorldPatch, String> {
    let Some(entry) = PATCH_TOOLS.iter().find(|entry| entry.name == name) else {
        return Err(format!("`{name}` is not a tool of this patch"));
    };
    let Ok(Value::Object(fields)) = serde_json::from_str::<Value>(arguments) else {
        return Err("arguments are not a JSON object".into());
    };
    match entry.shape {
        PatchToolShape::Session => {
            Err(format!("`{name}` ends or annotates a session; it decodes no patch item"))
        }
        PatchToolShape::Declare { variant, fixed } => {
            let mut value = fields;
            value.insert("type".into(), Value::String(variant.into()));
            for (key, fixed_value) in fixed {
                value.insert((*key).into(), Value::String((*fixed_value).into()));
            }
            let declaration = serde_json::from_value::<Declaration>(Value::Object(value))
                .map_err(|error| error.to_string())?;
            let mut patch = WorldPatch::default();
            if let Declaration::Fact(fact) = &declaration
                && let FactStandingRef::Canonical { evidence } = &fact.standing
            {
                patch.evidence.push(evidence.clone());
            }
            patch.declarations.push(declaration);
            Ok(patch)
        }
        PatchToolShape::Operate { variant } => {
            let mut value = fields;
            value.insert("op".into(), Value::String(variant.into()));
            let operation = serde_json::from_value::<ComponentOp>(Value::Object(value))
                .map_err(|error| error.to_string())?;
            let mut patch = WorldPatch::default();
            if let ComponentOp::Admit { evidence, .. } = &operation {
                patch.evidence.push(evidence.clone());
            }
            patch.operations.push(operation);
            Ok(patch)
        }
    }
}

/// The subject's own currently issued opportunity and its granted entries,
/// read from the snapshot alone. Never the caller's claim: PA.f41 refused a
/// forged `affordance_ids` on the Persona lane for the same reason — the
/// opportunity a consumer may act under is what the kernel most recently
/// derived, not anything it hands back to itself.
fn subject_opportunity<'a>(
    snapshot: &'a WorldSnapshot,
    subject: SubjectId,
) -> Option<(&'a DecisionOpportunity, Vec<AffordanceSnapshot>)> {
    let subject_row = snapshot.subjects.iter().find(|row| row.id == subject)?;
    let mut matching = snapshot
        .opportunities
        .iter()
        .filter(|opportunity| opportunity.scope.subject_id == subject);
    let opportunity = matching.next()?;
    if matching.next().is_some() {
        // More than one live opportunity for one subject is a kernel
        // invariant failure, not a choice for this reader to make.
        return None;
    }
    let granted: Vec<AffordanceSnapshot> = snapshot
        .affordances
        .iter()
        .filter(|entry| {
            subject_row.affordances.contains(&entry.id) && opportunity.affordance_ids.contains(&entry.id)
        })
        .cloned()
        .collect();
    if granted.is_empty() {
        return None;
    }
    Some((opportunity, granted))
}

/// The tool catalog for one subject's own granted affordances: an unknown
/// subject or one with no live opportunity gets an empty catalog rather than
/// an error — there is nothing for it to call. The table's own actor
/// vocabulary is the subject's granted entries only — `affordance_tools`,
/// never `catalog_tools`. `record_need` and `finish_without_proposal` end a
/// turn; that is the controller lane's own vocabulary, and
/// `decode_actor_call` has no arm for either, so offering them here would
/// hand the play agent a tool `decode_actor_call` always refuses (PA.f55).
pub fn actor_tools(prefix: &str, snapshot: &WorldSnapshot, subject: SubjectId) -> Vec<CodexToolDefinition> {
    match subject_opportunity(snapshot, subject) {
        Some((_, granted)) => controllers::affordance_tools(prefix, &granted),
        None => Vec::new(),
    }
}

/// One actor tool call, decoded against the subject's own current
/// opportunity: `decode_catalog_call` over the granted entry the tool names,
/// plus a verbatim optional `display` the generated schema does not carry
/// (the generic catalog tool has no display field; a play-agent call may add
/// one, decoded through the same canonical-text rule speech already uses).
/// The opportunity returned is the snapshot's own, never a value the caller
/// supplied, so Dungeon can commit it without ever having constructed one.
///
/// `prefix` is the same prefix `actor_tools` was called with: a name
/// `actor_tools` emits (`{prefix}{kind}`) decodes here as emitted, one
/// decoder rather than a caller stripping the prefix itself before this is
/// reached (PA.f55). An empty prefix strips nothing, so a bare `kind` still
/// decodes unchanged.
pub fn decode_actor_call(
    snapshot: &WorldSnapshot,
    subject: SubjectId,
    prefix: &str,
    kind: &str,
    arguments: &str,
) -> Result<(DecisionOpportunity, DecisionInvocation), String> {
    // A non-empty prefix must actually be carried: falling back to the bare
    // name here (PA.f71) let a name `actor_tools` never emits — because it
    // never printed the prefix `actor_tools` was called with — still decode.
    let kind = if prefix.is_empty() {
        kind
    } else {
        kind.strip_prefix(prefix)
            .ok_or_else(|| format!("`{kind}` does not carry the actor prefix `{prefix}`"))?
    };
    let (opportunity, granted) = subject_opportunity(snapshot, subject)
        .ok_or_else(|| "this subject holds no exact opportunity".to_owned())?;
    let entry = granted
        .iter()
        .find(|entry| entry.entry.kind.0 == kind)
        .ok_or_else(|| format!("`{kind}` is not granted by this subject's opportunity"))?;
    let mut invocation = controllers::decode_catalog_call(entry, arguments)?;
    if let Ok(Value::Object(fields)) = serde_json::from_str::<Value>(arguments)
        && let Some(display) = fields.get("display")
    {
        let display = display
            .as_str()
            .ok_or_else(|| "`display` is not a string".to_owned())?;
        invocation.display =
            Some(Statement::new(display).ok_or_else(|| "`display` is not canonical text".to_owned())?);
    }
    Ok((opportunity.clone(), invocation))
}

/// The agent-facing refusal: full detail. `ActionRejected` names the entry
/// precondition, slot, or role that failed, with the labels of the roles the
/// actor's own invocation bound when `entry` and `invocation` are given;
/// `PatchRejected` names the tool call — by the handle it declared, by its
/// position among the operation calls, or, when `batch` is the
/// `DecodedBatch` that produced the rejected patch, by the exact call index
/// `batch`'s own site map attributes it to. Every other `KernelError` has
/// nothing more useful to say than its own `Display`.
///
/// This is the agent's own surface, never the actor's: it names ids,
/// statements, and other world detail the actor's own attempt did not
/// reveal. `describe_refusal_to_actor` is the narrow twin that a subject may
/// see.
pub fn describe_refusal(
    snapshot: &WorldSnapshot,
    entry: Option<&AffordanceSnapshot>,
    invocation: Option<&DecisionInvocation>,
    batch: Option<&DecodedBatch>,
    error: &KernelError,
) -> String {
    match error {
        KernelError::ActionRejected(mismatches) => mismatches
            .iter()
            .map(|mismatch| describe_action_mismatch(mismatch, snapshot, entry, invocation))
            .collect::<Vec<_>>()
            .join("; "),
        KernelError::PatchRejected(mismatches) => mismatches
            .iter()
            .map(|mismatch| describe_patch_mismatch(snapshot, mismatch, batch))
            .collect::<Vec<_>>()
            .join("; "),
        other => other.to_string(),
    }
}

/// The actor-facing refusal: only what the actor's own attempt already
/// reveals. For `ActionRejected`, only the failed preconditions, and only by
/// the bare names of the roles `entry` declares — never a resolved label, an
/// id, or a fact's statement, all of which the actor's own call did not
/// hand back to it. Every other `KernelError` — `MissingApprovals`, a store
/// or journal error, anything else — returns one fixed line that names
/// nothing, because none of those reveal anything about the actor's own
/// attempt at all.
pub fn describe_refusal_to_actor(entry: &AffordanceSnapshot, error: &KernelError) -> String {
    const REFUSED: &str = "refused";
    let KernelError::ActionRejected(mismatches) = error else {
        return REFUSED.to_owned();
    };
    let described: Vec<String> = mismatches
        .iter()
        .filter_map(|mismatch| describe_action_mismatch_to_actor(mismatch, entry))
        .collect();
    if described.is_empty() {
        REFUSED.to_owned()
    } else {
        described.join("; ")
    }
}

/// The precondition-numbered `ActionMismatch` variants, alongside the
/// precondition index each one names into `entry.entry.preconditions`.
fn precondition_index(mismatch: &ActionMismatch) -> Option<usize> {
    match mismatch {
        ActionMismatch::ActorNotPresent { precondition }
        | ActionMismatch::TargetUnreachable { precondition }
        | ActionMismatch::InsufficientHolding { precondition }
        | ActionMismatch::NotAuthorized { precondition }
        | ActionMismatch::NoStanding { precondition }
        | ActionMismatch::FactUnknown { precondition }
        | ActionMismatch::NoAudience { precondition }
        | ActionMismatch::CannotReach { precondition }
        | ActionMismatch::NotCommitted { precondition } => Some(*precondition),
        _ => None,
    }
}

/// The role(s) one `Precondition` names, in the order its own fields declare
/// them. `HasStanding` names none: a grievance kind is not a role.
/// `CanBroadcast` and `CanReach` name the channel role their `via` binds when
/// it is `AudienceSpec::Channel` — `AudienceSpec::Colocated` names no role,
/// since co-location is derived from position, not bound (PA.f72: this used
/// to drop the channel role of both entirely).
fn precondition_roles(precondition: &patch::Precondition) -> Vec<patch::Role> {
    match precondition {
        patch::Precondition::Present { at } => vec![at.clone()],
        patch::Precondition::Reachable { to, .. } => vec![to.clone()],
        patch::Precondition::Holds { resource, .. } => vec![resource.clone()],
        patch::Precondition::Authorized { over, .. } => vec![over.clone()],
        patch::Precondition::HasStanding { .. } => Vec::new(),
        patch::Precondition::Knows { fact, .. } => vec![fact.clone()],
        patch::Precondition::CanBroadcast { via } => audience_role(via),
        patch::Precondition::CanReach { subject, via } => {
            let mut roles = vec![subject.clone()];
            roles.extend(audience_role(via));
            roles
        }
        patch::Precondition::Committed { to, .. } => vec![to.clone()],
    }
}

/// The role an `AudienceSpec` binds, if any: `Channel` names the role its
/// invocation must bind a channel to; `Colocated` names none.
fn audience_role(via: &patch::AudienceSpec) -> Vec<patch::Role> {
    match via {
        patch::AudienceSpec::Colocated => Vec::new(),
        patch::AudienceSpec::Channel(role) => vec![role.clone()],
    }
}

/// The agent-facing precondition context: for each role the failed
/// precondition names, the role's own name and the label of what the
/// actor's invocation actually bound it to (PA.f56 — `ActionRejected` never
/// read the entry's preconditions before this). `None` when `entry` or
/// `invocation` is not available to the caller, or the precondition names no
/// role.
fn precondition_context(
    snapshot: &WorldSnapshot,
    entry: Option<&AffordanceSnapshot>,
    invocation: Option<&DecisionInvocation>,
    index: usize,
) -> Option<String> {
    let entry = entry?;
    let invocation = invocation?;
    let precondition = entry.entry.preconditions.get(index)?;
    let roles = precondition_roles(precondition);
    if roles.is_empty() {
        return None;
    }
    let labels: Vec<String> = roles
        .iter()
        .filter_map(|role| {
            let binding = invocation.bindings.iter().find(|binding| binding.role.0 == role.0)?;
            Some(format!(
                "role `{}` bound to {}",
                role.0,
                describe_target(snapshot, &binding.target)
            ))
        })
        .collect();
    if labels.is_empty() {
        None
    } else {
        Some(format!(" ({})", labels.join(", ")))
    }
}

fn describe_target(snapshot: &WorldSnapshot, target: &Target) -> String {
    let id = match target {
        Target::Subject(id) => id_text(*id),
        Target::Entity(id) => id_text(*id),
        Target::Edge(id) => id_text(*id),
    };
    label_for_any_id(snapshot, &id).unwrap_or_else(|| format!("[{id}]"))
}

/// A `RefKind` in plain words, not Rust `Debug`.
fn render_ref_kind(kind: RefKind) -> &'static str {
    match kind {
        RefKind::Subject(_) => "subject",
        RefKind::Entity(super::EntityKind::Place) => "place",
        RefKind::Entity(super::EntityKind::Resource) => "resource",
        RefKind::Entity(super::EntityKind::Fact) => "fact",
        RefKind::Entity(super::EntityKind::Channel) => "channel",
        RefKind::Edge(_) => "route",
        RefKind::Affordance => "affordance",
    }
}

fn describe_action_mismatch(
    mismatch: &ActionMismatch,
    snapshot: &WorldSnapshot,
    entry: Option<&AffordanceSnapshot>,
    invocation: Option<&DecisionInvocation>,
) -> String {
    let context =
        |mismatch: &ActionMismatch| precondition_index(mismatch)
            .and_then(|index| precondition_context(snapshot, entry, invocation, index))
            .unwrap_or_default();
    match mismatch {
        ActionMismatch::UnboundRole { role } => format!("role `{}` was never bound", role.0),
        ActionMismatch::UnknownRole { role } => {
            format!("role `{}` names no role the entry declares", role.0)
        }
        ActionMismatch::DuplicateRoleBinding { role } => {
            format!("role `{}` was bound twice", role.0)
        }
        ActionMismatch::UnknownTarget { role } => {
            format!("role `{}` names a target the world does not have", role.0)
        }
        ActionMismatch::TargetKindMismatch {
            role,
            expected,
            actual,
        } => format!(
            "role `{}` expected a {} but was bound to a {}",
            role.0,
            render_ref_kind(*expected),
            render_ref_kind(*actual)
        ),
        ActionMismatch::ActorNotPresent { precondition } => format!(
            "precondition #{precondition} of the entry failed: the actor was not present{}",
            context(mismatch)
        ),
        ActionMismatch::TargetUnreachable { precondition } => format!(
            "precondition #{precondition} of the entry failed: the target is unreachable{}",
            context(mismatch)
        ),
        ActionMismatch::InsufficientHolding { precondition } => format!(
            "precondition #{precondition} of the entry failed: the actor does not hold enough{}",
            context(mismatch)
        ),
        ActionMismatch::SlotNotProposed { slot } => {
            format!("effect slot #{slot} was not proposed")
        }
        ActionMismatch::UnknownSlot { slot } => {
            format!("effect slot #{slot} names no slot the entry declares")
        }
        ActionMismatch::DuplicateSlotProposal { slot } => {
            format!("effect slot #{slot} was proposed twice")
        }
        ActionMismatch::MagnitudeShapeMismatch { slot } => {
            format!("effect slot #{slot}'s magnitude is the wrong shape")
        }
        ActionMismatch::MagnitudeOverCeiling { slot } => {
            format!("effect slot #{slot}'s magnitude is over its ceiling")
        }
        ActionMismatch::ZeroMagnitude { slot } => {
            format!("effect slot #{slot}'s magnitude is zero")
        }
        ActionMismatch::SpeechRequired => {
            "the entry requires speech and none was carried".to_owned()
        }
        ActionMismatch::SpeechNotCarried => {
            "this entry carries no speech but an utterance was proposed".to_owned()
        }
        ActionMismatch::EmptySpeech => "the proposed utterance is not canonical text".to_owned(),
        ActionMismatch::EmptyDisplay => "the proposed display is not canonical text".to_owned(),
        ActionMismatch::NotAuthorized { precondition } => format!(
            "precondition #{precondition} of the entry failed: the actor is not authorized over that target{}",
            context(mismatch)
        ),
        ActionMismatch::NoStanding { precondition } => format!(
            "precondition #{precondition} of the entry failed: no forum's standing reaches the actor{}",
            context(mismatch)
        ),
        ActionMismatch::ActorRoleBound => {
            "the invocation bound the reserved `actor` role".to_owned()
        }
        ActionMismatch::DelegationNotMonotone { slot } => format!(
            "effect slot #{slot} would grant authority the actor does not itself hold"
        ),
        ActionMismatch::FactUnknown { precondition } => format!(
            "precondition #{precondition} of the entry failed: the actor does not hold that fact{}",
            context(mismatch)
        ),
        ActionMismatch::NoAudience { precondition } => format!(
            "precondition #{precondition} of the entry failed: the actor has no audience{}",
            context(mismatch)
        ),
        ActionMismatch::CannotReach { precondition } => format!(
            "precondition #{precondition} of the entry failed: the addressed subject is not in that audience{}",
            context(mismatch)
        ),
        ActionMismatch::NotCommitted { precondition } => format!(
            "precondition #{precondition} of the entry failed: the actor holds no such commitment{}",
            context(mismatch)
        ),
    }
}

/// The actor-facing rendering of one `ActionMismatch`: `None` for anything
/// that is not a failed precondition (a role/slot mismatch names the actor's
/// own call shape back to it and is not withheld in principle, but PA.f57's
/// ruling scopes `describe_refusal_to_actor` to failed preconditions alone,
/// so every other variant is silent here). A precondition's role names are
/// bare — `entry.entry.preconditions[index]`'s own `Role`s, never resolved
/// to a label, an id, or a fact's statement: the actor already knows what it
/// bound there, and nothing else is handed back.
fn describe_action_mismatch_to_actor(mismatch: &ActionMismatch, entry: &AffordanceSnapshot) -> Option<String> {
    let index = precondition_index(mismatch)?;
    let roles = entry
        .entry
        .preconditions
        .get(index)
        .map(precondition_roles)
        .unwrap_or_default();
    // A precondition that binds no role (`HasStanding`, `CanBroadcast` over
    // `Colocated`) or whose index does not resolve gets plain wording of its
    // own, rather than being spliced into the templated "role `X`" sentence
    // below — PA.f72's garble read "a precondition over role a precondition
    // with no role, which you bound yourself, failed".
    Some(if roles.is_empty() {
        "a precondition you bound no role for failed".to_owned()
    } else {
        let role_text = roles
            .iter()
            .map(|role| format!("`{}`", role.0))
            .collect::<Vec<_>>()
            .join(", ");
        format!("a precondition over role {role_text}, which you bound yourself, failed")
    })
}

/// A `Mismatch`'s own site, read generically off its serialized shape rather
/// than through a fifty-arm match: every variant tags its site under one of
/// `site`, `operation`, `position`, `handle`, `resource`, or `referent`, and
/// this reads whichever is present. `WrongKind` and `CustodyNotConserved`
/// carry more than a bare site (the kinds compared, or the resource whose
/// ledger failed to balance) and are rendered fully before the generic
/// reader ever runs; `EmptyEvidence`'s `position` is an index into the
/// patch's evidence list, not a declaration index, so it is also rendered
/// before the generic reader could conflate the two (PA.f56). `site` and
/// `operation` name the offending tool call by the exact call index
/// `batch`'s own site map attributes it to, when `batch` is given; falling
/// back to the raw declaration/operation index otherwise, since that index
/// is still the best available answer with no batch in hand.
fn describe_patch_mismatch(
    snapshot: &WorldSnapshot,
    mismatch: &Mismatch,
    batch: Option<&DecodedBatch>,
) -> String {
    let value = serde_json::to_value(mismatch).unwrap_or(Value::Null);
    let kind = value
        .get("mismatch")
        .and_then(Value::as_str)
        .unwrap_or("mismatch")
        .to_owned();
    let Some(object) = value.as_object() else {
        return kind;
    };
    if kind == "wrong_kind" {
        let site = object
            .get("site")
            .map(|site| describe_site(site, batch))
            .unwrap_or_else(|| "an unresolved site".to_owned());
        let referent = object
            .get("referent")
            .map(|referent| describe_ref_name(snapshot, referent))
            .unwrap_or_else(|| "a reference".to_owned());
        let expected = render_ref_kind_value(object.get("expected"));
        let actual = render_ref_kind_value(object.get("actual"));
        return format!("{site}: `{referent}` names a {actual} but a {expected} was expected");
    }
    if kind == "custody_not_conserved" {
        let resource = object
            .get("resource")
            .map(|referent| describe_ref_name(snapshot, referent))
            .unwrap_or_else(|| "a resource".to_owned());
        return format!("the candidate ledger does not balance for `{resource}`");
    }
    if kind == "empty_evidence" {
        let position = object.get("position").and_then(Value::as_u64).unwrap_or_default();
        // PA.f73: `evidence_calls[position]` names the exact call that cited
        // this evidence entry, the same way `site_calls` names a declaration
        // or operation's call; with no batch in hand there is no call to
        // name, so this stays the bare evidence-item description.
        let call = batch
            .and_then(|batch| batch.evidence_calls.get(position as usize))
            .map(|call_index| format!("call #{call_index}: "))
            .unwrap_or_default();
        return format!("{call}evidence item #{position}: an empty or duplicate evidence reference");
    }
    if kind == "unresolved_draft" {
        // PA.f73: the generic reader below drops `referent` and `expected`
        // for every kind it does not special-case; `UnresolvedDraft` needs
        // both — the handle that never resolved, and what it was expected to
        // resolve to.
        let site = object
            .get("site")
            .map(|site| describe_site(site, batch))
            .unwrap_or_else(|| "an unresolved site".to_owned());
        let handle = object.get("referent").and_then(Value::as_str).unwrap_or("?");
        let expected = render_ref_kind_value(object.get("expected"));
        return format!("{site}: draft handle `{handle}` was never resolved to a {expected}");
    }
    if kind == "duplicate_handle" {
        // PA.f73: naming only "the call that declared `handle`" is
        // ambiguous for a handle declared twice — that is exactly what
        // `DuplicateHandle` reports. Name every call that declared it.
        let handle = object.get("handle").and_then(Value::as_str).unwrap_or("?");
        return format!(
            "duplicate handle `{handle}`, declared by {}",
            describe_duplicate_handle_sites(handle, batch)
        );
    }
    let site = if let Some(site) = object.get("site") {
        Some(describe_site(site, batch))
    } else if let Some(operation) = object.get("operation").and_then(Value::as_u64) {
        Some(describe_operation_site(operation as usize, batch))
    } else if let Some(position) = object.get("position").and_then(Value::as_u64) {
        // Only `EmptyHandle` reaches here now: `empty_evidence`'s `position`
        // returned above, so this index is always a declaration index.
        Some(describe_declaration_site(position as usize, batch))
    } else if let Some(handle) = object.get("handle").and_then(Value::as_str) {
        Some(format!("the call that declared `{handle}`"))
    } else if let Some(referent) = object.get("referent") {
        Some(describe_referent(referent))
    } else {
        None
    };
    match site {
        Some(site) => format!("{site}: {kind}"),
        None => kind,
    }
}

fn describe_site(site: &Value, batch: Option<&DecodedBatch>) -> String {
    match site.get("site").and_then(Value::as_str) {
        Some("declaration") => {
            let handle = site.get("at").and_then(Value::as_str).unwrap_or("?");
            format!("the call that declared `{handle}`")
        }
        Some("operation") => {
            let index = site.get("at").and_then(Value::as_u64).unwrap_or_default();
            describe_operation_site(index as usize, batch)
        }
        _ => "an unresolved site".to_owned(),
    }
}

/// An operation site, named by the exact call index `batch`'s site map
/// attributes it to when `batch` is given, or by its raw index into the
/// patch's own operations otherwise.
fn describe_operation_site(index: usize, batch: Option<&DecodedBatch>) -> String {
    match batch.and_then(|batch| batch.site_calls.get(&PatchSite::Operation(index))) {
        Some(call_index) => format!("call #{call_index}"),
        None => format!("operation tool call #{index}"),
    }
}

/// A declaration site, named the same way `describe_operation_site` names an
/// operation site.
fn describe_declaration_site(index: usize, batch: Option<&DecodedBatch>) -> String {
    match batch.and_then(|batch| batch.site_calls.get(&PatchSite::Declaration(index))) {
        Some(call_index) => format!("call #{call_index}"),
        None => format!("declaration tool call #{index}"),
    }
}

/// Every call that declared `handle`, unambiguously (PA.f73): a
/// `DuplicateHandle` mismatch carries only the handle text, not a site, so
/// naming "the call that declared `handle`" is ambiguous by construction
/// when the handle was declared twice. With `batch` in hand, every
/// declaration in `batch.patch.declarations` whose own `handle` field
/// matches is named by its exact call, through the same site map
/// `describe_declaration_site` reads. With no batch, this falls back to the
/// same single-site wording every other handle-only mismatch used before.
fn describe_duplicate_handle_sites(handle: &str, batch: Option<&DecodedBatch>) -> String {
    let Some(batch) = batch else {
        return format!("the call that declared `{handle}`");
    };
    let sites: Vec<String> = batch
        .patch
        .declarations
        .iter()
        .enumerate()
        .filter(|(_, declaration)| {
            serde_json::to_value(*declaration)
                .ok()
                .and_then(|value| value.get("handle").and_then(Value::as_str).map(str::to_owned))
                .as_deref()
                == Some(handle)
        })
        .map(|(index, _)| describe_declaration_site(index, Some(batch)))
        .collect();
    if sites.is_empty() {
        format!("the call that declared `{handle}`")
    } else {
        sites.join(" and ")
    }
}

/// A `RefKind` read off its serialized `{"namespace": ..., "kind": ...}`
/// shape, in the same plain words `render_ref_kind` renders the typed value.
fn render_ref_kind_value(value: Option<&Value>) -> String {
    let Some(value) = value else {
        return "reference".to_owned();
    };
    match value.get("namespace").and_then(Value::as_str) {
        Some("subject") => "subject".to_owned(),
        Some("entity") => value
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("entity")
            .to_owned(),
        Some("edge") => "route".to_owned(),
        Some("affordance") => "affordance".to_owned(),
        _ => "reference".to_owned(),
    }
}

/// A bare draft handle referent (a plain string, quoted as the call wrote
/// it). `ContainmentCycle` and `RouteSelfLoop` are the only `Mismatch`
/// variants that reach this path, and both carry a `DraftHandle`, never a
/// `RefName` — `WrongKind`'s own `RefName` referent is rendered by
/// `describe_ref_name` instead, from `describe_patch_mismatch`'s dedicated
/// arm, because `WrongKind` also carries a `site` that the generic reader
/// would otherwise match first, which made the `RefName` half of this
/// function unreachable dead code.
fn describe_referent(referent: &Value) -> String {
    let handle = referent.as_str().unwrap_or("?");
    format!("the reference to `{handle}`")
}

/// A `RefName` referent (a tagged `{namespace, ref}`), resolved to the
/// standing thing's label when the reference is canonical — the one place a
/// `WrongKind` mismatch's referent is turned into the same labels
/// `table_view` prints, rather than a second lookup.
fn describe_ref_name(snapshot: &WorldSnapshot, referent: &Value) -> String {
    let namespace = referent
        .get("namespace")
        .and_then(Value::as_str)
        .unwrap_or("reference");
    let reference = referent.get("ref");
    let is_existing = reference
        .and_then(|value| value.get("ref"))
        .and_then(Value::as_str)
        == Some("existing");
    if is_existing
        && let Some(id) = reference
            .and_then(|value| value.get("value"))
            .and_then(Value::as_str)
    {
        return label_for_any_id(snapshot, id)
            .unwrap_or_else(|| format!("the {namespace} reference [{id}]"));
    }
    format!("a draft {namespace} reference")
}

/// The label of whatever standing thing carries this canonical id, searched
/// across every category `table_view` renders, because a `RefName` does not
/// say which namespace's referent it is once flattened to a bare id string.
fn label_for_any_id(snapshot: &WorldSnapshot, id: &str) -> Option<String> {
    snapshot
        .places
        .iter()
        .find(|place| id_text(place.id) == id)
        .map(|place| format!("{} [{}]", place.label, id))
        .or_else(|| {
            snapshot
                .subjects
                .iter()
                .find(|subject| id_text(subject.id) == id)
                .map(|subject| format!("{} [{}]", subject.label, id))
        })
        .or_else(|| {
            snapshot
                .resources
                .iter()
                .find(|resource| id_text(resource.id) == id)
                .map(|resource| format!("{} [{}]", resource.label, id))
        })
        .or_else(|| {
            snapshot
                .routes
                .iter()
                .find(|route| id_text(route.id) == id)
                .map(|route| format!("{} [{}]", route.label, id))
        })
        .or_else(|| {
            snapshot
                .affordances
                .iter()
                .find(|affordance| id_text(affordance.id) == id)
                .map(|affordance| format!("{} [{}]", affordance.entry.kind.0, id))
        })
}

/// A route's access, in words rather than `AccessKind`'s `Debug`.
fn render_access(access: &AccessKind) -> String {
    match access {
        AccessKind::Public => "public".to_owned(),
        AccessKind::Restricted { requires } => format!("restricted, requires `{}`", requires.0),
    }
}

/// A subject's kind, in words rather than `SubjectKind`'s `Debug`.
fn render_subject_kind(kind: SubjectKind) -> &'static str {
    match kind {
        SubjectKind::Person => "person",
        SubjectKind::Institution => "institution",
        SubjectKind::Population => "population",
    }
}

/// A subject's controller mode, in words rather than `ControllerMode`'s
/// `Debug`.
fn render_controller_mode(mode: ControllerMode) -> &'static str {
    match mode {
        ControllerMode::Human => "human",
        ControllerMode::NarrativePersona => "narrative persona",
        ControllerMode::OperationalAgent => "operational agent",
    }
}

/// A fact's standing, as the omniscient play table reads it: `Ruled` prints
/// as `ruled`, not `canonical` — the collapse that every subject-facing
/// knowledge row applies never reaches this reader.
fn render_fact_standing(snapshot: &WorldSnapshot, standing: &super::FactStandingView) -> String {
    match standing {
        super::FactStandingView::Canonical => "canonical".to_owned(),
        super::FactStandingView::Ruled => "ruled".to_owned(),
        super::FactStandingView::Claimed { by } => {
            let label = snapshot
                .subjects
                .iter()
                .find(|holder| holder.id == *by)
                .map_or_else(|| "an unknown subject".to_owned(), |holder| holder.label.clone());
            format!("claimed by {label}")
        }
    }
}

/// A `CommitmentKind`, in words rather than `Debug` (PA.f74: this used to
/// print as `Obligation`, `Routine`, or `Goal`).
fn render_commitment_kind(kind: patch::CommitmentKind) -> &'static str {
    match kind {
        patch::CommitmentKind::Routine => "routine",
        patch::CommitmentKind::Obligation => "obligation",
        patch::CommitmentKind::Goal => "goal",
    }
}

/// A joined, sorted list, or `"none"` when empty — the same rule
/// `table_view`'s own `joined` closure applies, factored out so the
/// authority/office/forum renderers below can share it without capturing
/// `table_view`'s locals.
fn joined_sorted(mut items: Vec<String>) -> String {
    if items.is_empty() {
        "none".to_owned()
    } else {
        items.sort();
        items.join(", ")
    }
}

/// What an `AuthorityTarget` covers, by id and label: a subject, or a
/// place's whole subtree.
fn render_authority_target(snapshot: &WorldSnapshot, target: &patch::AuthorityTarget) -> String {
    match target {
        patch::AuthorityTarget::Subject(id) => {
            label_for_any_id(snapshot, &id_text(*id)).unwrap_or_else(|| format!("[{}]", id_text(*id)))
        }
        patch::AuthorityTarget::PlaceSubtree(id) => format!(
            "the subtree of {}",
            label_for_any_id(snapshot, &id_text(*id)).unwrap_or_else(|| format!("[{}]", id_text(*id)))
        ),
    }
}

/// One authority grant, by its kind name and what it covers — what
/// `Precondition::Authorized`, `create_commitment`'s own `checks`, and a
/// restricted route's `AccessKind::Restricted { requires }` all read against
/// (PA.f74).
fn render_authority_grant(snapshot: &WorldSnapshot, grant: &patch::AuthorityGrant) -> String {
    format!("`{}` over {}", grant.kind.0, render_authority_target(snapshot, &grant.over))
}

/// One office: its canonical name, the institution that constitutes it, its
/// incumbent (or vacancy), and what it lends — every field `create_commitment`
/// and `Precondition::Authorized` may resolve through delegation (PA.f74).
fn render_office(snapshot: &WorldSnapshot, office: &super::OfficeSnapshot) -> String {
    let institution = label_for_any_id(snapshot, &id_text(office.institution))
        .unwrap_or_else(|| format!("[{}]", id_text(office.institution)));
    let incumbent = office.incumbent.map_or_else(
        || "vacant".to_owned(),
        |id| label_for_any_id(snapshot, &id_text(id)).unwrap_or_else(|| format!("[{}]", id_text(id))),
    );
    let lends = joined_sorted(
        office
            .authority
            .iter()
            .map(|grant| render_authority_grant(snapshot, grant))
            .collect(),
    );
    format!("`{}` of {institution} (incumbent: {incumbent}, lends: {lends})", office.office.0)
}

/// One forum: the grievance kind it takes, and who sits it — what
/// `Precondition::HasStanding` reads against (PA.f74).
fn render_forum(snapshot: &WorldSnapshot, forum: &super::ForumSnapshot) -> String {
    format!(
        "`{}` at {}",
        forum.grievance.0,
        label_for_any_id(snapshot, &id_text(forum.forum)).unwrap_or_else(|| format!("[{}]", id_text(forum.forum)))
    )
}

/// A dependency target, by the id of the thing depended on, so a call
/// composing `bind`/`release`'s `DependencyRef` or reading `advance_pressure`
/// /`reduce_pressure`'s `PressureSourceRef::Dependency` can name it back.
fn render_dependency_target(snapshot: &WorldSnapshot, target: &DependencyTarget) -> String {
    match target {
        DependencyTarget::Resource(id) => format!(
            "resource {}",
            label_for_any_id(snapshot, &id_text(*id)).unwrap_or_else(|| format!("[{}]", id_text(*id)))
        ),
        DependencyTarget::Route(id) => format!(
            "route {}",
            label_for_any_id(snapshot, &id_text(*id)).unwrap_or_else(|| format!("[{}]", id_text(*id)))
        ),
        DependencyTarget::Subject(id) => format!(
            "subject {}",
            label_for_any_id(snapshot, &id_text(*id)).unwrap_or_else(|| format!("[{}]", id_text(*id)))
        ),
    }
}

/// A pressure source, in the same shape `advance_pressure`/`reduce_pressure`'s
/// `PressureSourceRef` composite takes: a commitment names the promisor
/// subject id and the commitment key; a dependency names its target; a
/// subject names itself.
fn render_pressure_source(snapshot: &WorldSnapshot, source: &PressureSource) -> String {
    match source {
        PressureSource::Commitment { subject, key } => format!(
            "commitment {}/{} of {}",
            id_text(key.command),
            key.index,
            label_for_any_id(snapshot, &id_text(*subject))
                .unwrap_or_else(|| format!("[{}]", id_text(*subject)))
        ),
        PressureSource::Dependency(target) => render_dependency_target(snapshot, target),
        PressureSource::Subject(id) => label_for_any_id(snapshot, &id_text(*id))
            .unwrap_or_else(|| format!("subject [{}]", id_text(*id))),
    }
}

/// A channel's reach, in words: either the exact subjects it carries to, or
/// the place whose occupants it carries to.
fn render_reach(snapshot: &WorldSnapshot, reach: &Reach) -> String {
    match reach {
        Reach::Subjects(subjects) => {
            if subjects.is_empty() {
                "nobody (silenced)".to_owned()
            } else {
                let mut labels: Vec<String> = subjects
                    .iter()
                    .map(|id| {
                        label_for_any_id(snapshot, &id_text(*id))
                            .unwrap_or_else(|| format!("[{}]", id_text(*id)))
                    })
                    .collect();
                labels.sort();
                labels.join(", ")
            }
        }
        Reach::Place(place) => format!(
            "everyone in {}",
            label_for_any_id(snapshot, &id_text(*place))
                .unwrap_or_else(|| format!("[{}]", id_text(*place)))
        ),
    }
}

impl InferenceRequest {
    /// The play agent's own request shape: `tool_request` under
    /// `InferencePurpose::Play`, with parallel calls on so one round can
    /// carry more than one authoring or actor call.
    pub fn play(
        command_id: CommandId,
        round: usize,
        model: &str,
        instructions: &str,
        input: Vec<CodexInputItem>,
        tools: Vec<CodexToolDefinition>,
        max_output_tokens: u32,
    ) -> Result<InferenceRequest, ControllerError> {
        tool_request(
            command_id,
            round,
            InferencePurpose::Play,
            model,
            instructions,
            input,
            tools,
            RequestShape {
                max_output_tokens,
                parallel_tool_calls: true,
            },
        )
    }
}

impl InferenceOutput {
    /// The events one inference produced, read rather than matched apart:
    /// Dungeon composes an agent loop over these without depending on the
    /// controller lane's own accumulator types.
    pub fn events(&self) -> &[InferenceEvent] {
        &self.events
    }

    /// The provider's own receipt digest, carried verbatim.
    pub fn receipt_digest(&self) -> &str {
        &self.receipt_digest
    }
}

/// The canonical id as the patch vocabulary spells it: the bare UUID text,
/// without the typed wrapper's name. Moved here from the seed lane, which
/// still calls it here; nothing about its behaviour changed with the move.
/// `pub`, not `pub(super)` (PA.f113): this is the one place that decides how
/// an id is spelled — `table_view` prints through it, and Dungeon's
/// `dispatch` reads an id an agent typed back through `id_text_matches`
/// below, rather than each consumer keeping its own debug-paren-strip copy.
/// A copy drifts silently when this printer changes; a shared call site
/// cannot.
pub fn id_text(id: impl std::fmt::Debug) -> String {
    let text = format!("{id:?}");
    match (text.find('('), text.rfind(')')) {
        (Some(open), Some(close)) if open < close => text[open + 1..close].to_owned(),
        _ => text,
    }
}

/// The parser matching `id_text` above (PA.f113): `text` names `id` only
/// when it is exactly the bytes `id_text(id)` would print — no brackets, no
/// padding, no case folding, because this is plain string equality against
/// the printer's own output rather than a second, independently-tolerant
/// parse. Kept beside the printer so the two cannot drift apart the way
/// `table_view`'s print and Dungeon's three hand-copied strips once did.
pub fn id_text_matches(id: impl std::fmt::Debug, text: &str) -> bool {
    id_text(id) == text
}

/// The world the seed session is authoring into, rendered from snapshot
/// fields that already exist. Moved from the elaboration lane, byte-for-byte:
/// the seed prompt calls it here now, and its text does not change with the
/// move. Seed-only; `table_view` below is the play agent's own, richer
/// renderer over the same snapshot and does not build on this one, so a
/// change to one never reaches the other's bytes.
pub(super) fn render_world_structure(snapshot: &WorldSnapshot) -> String {
    let place_label = |id: EntityId| {
        snapshot
            .places
            .iter()
            .find(|entry| entry.id == id)
            .map_or_else(
                || "an unnamed place".to_owned(),
                |entry| entry.label.clone(),
            )
    };
    // Ids are printed beside labels because a patch names an existing thing
    // by its canonical id and nothing else; a model that sees only labels
    // can only guess, and the first live seed session guessed six times.
    let mut out = String::from(
        "Standing structure (reference an existing thing by the id in brackets, \
         exactly as printed; reference a thing declared in this patch by its handle):\n",
    );
    out.push_str("  Places:");
    if snapshot.places.is_empty() {
        out.push_str(" none");
    }
    for place in &snapshot.places {
        match place.container {
            Some(container) => {
                out.push_str(&format!(
                    " {} [{}] (in {});",
                    place.label,
                    id_text(place.id),
                    place_label(container)
                ));
            }
            None => out.push_str(&format!(" {} [{}];", place.label, id_text(place.id))),
        }
    }
    out.push_str("\n  Routes:");
    if snapshot.routes.is_empty() {
        out.push_str(" none");
    }
    for route in &snapshot.routes {
        out.push_str(&format!(
            " {} [{}]: {} -> {}, {:?}, {};",
            route.label,
            id_text(route.id),
            place_label(route.from),
            place_label(route.to),
            route.access,
            if route.open { "open" } else { "closed" }
        ));
    }
    out.push_str("\n  Subjects:");
    if snapshot.subjects.is_empty() {
        out.push_str(" none");
    }
    for subject in &snapshot.subjects {
        out.push_str(&format!(
            " {} [{}] ({:?}, {}, in {}, grants: {}, {});",
            subject.label,
            id_text(subject.id),
            subject.kind,
            subject
                .controller_mode
                .map_or_else(|| "external".to_owned(), |mode| format!("{mode:?}")),
            subject
                .position
                .map_or_else(|| "nowhere".to_owned(), place_label),
            subject.affordances.len(),
            if subject.qualified {
                "counts"
            } else {
                "does not count"
            }
        ));
    }
    out.push_str("\n  Affordances:");
    if snapshot.affordances.is_empty() {
        out.push_str(" none");
    }
    for affordance in &snapshot.affordances {
        out.push_str(&format!(
            " {} [{}], roles: {}, {};",
            affordance.entry.kind.0,
            id_text(affordance.id),
            if affordance.entry.roles.is_empty() {
                "none".to_owned()
            } else {
                affordance
                    .entry
                    .roles
                    .iter()
                    .map(|role| role.role.0.clone())
                    .collect::<Vec<_>>()
                    .join("/")
            },
            if affordance.entry.carries_speech {
                "speech"
            } else {
                "silent"
            }
        ));
    }
    out.push_str("\n  Resources:");
    if snapshot.resources.is_empty() {
        out.push_str(" none");
    }
    for resource in &snapshot.resources {
        out.push_str(&format!(" {};", resource.label));
    }
    out.push_str("\n  Shortfall rows:");
    if snapshot.scale_deficit.is_empty() {
        out.push_str(" none");
    }
    for row in &snapshot.scale_deficit {
        out.push_str(&format!(
            " {} {}: target {}, qualified {}, short {};",
            super::elaboration::render_jurisdiction(snapshot, row.jurisdiction),
            super::elaboration::render_kind(row.kind),
            row.target,
            row.qualified,
            row.deficit
        ));
    }
    out.push('\n');
    out
}

/// The play agent's whole-world view. The agent holds `Play`, so it may see
/// everything, including standing and secrets no subject-facing surface
/// shows: standing is printed here and nowhere subject-facing. It grows from
/// `render_world_structure` in what it covers — every subject's place, mode,
/// retired flag, granted entry names, holdings, and dependencies; the
/// places' occupants; every declared channel; every fact the world holds —
/// including one nobody currently perceives — with its id, its real standing,
/// and who knows it; every commitment by its key, every pressure row, and
/// every subject's persona material — because `PLAY_TOOLS` takes all of
/// those ids. It is its own renderer, not a wrapper over
/// `render_world_structure`, so the seed prompt's bytes never move when this
/// one changes.
///
/// `WorldSnapshot::facts` and `WorldSnapshot::channels` are built straight
/// from world state in `lib.rs`'s `snapshot`, independent of any subject's
/// own knowledge projection, so this reader sees a `Ruled` fact as `ruled`
/// even though `FactStanding::Ruled` still collapses to
/// `FactStandingView::Canonical` on every subject-facing knowledge row (the
/// Projector and the typed view): a subject's own perception cannot tell a
/// ruling from an ordinarily canonical fact, but the play table, which holds
/// `Play`, can.
pub fn table_view(snapshot: &WorldSnapshot) -> String {
    let place_label = |id: EntityId| {
        snapshot
            .places
            .iter()
            .find(|entry| entry.id == id)
            .map_or_else(
                || "an unnamed place".to_owned(),
                |entry| entry.label.clone(),
            )
    };
    let joined = joined_sorted;

    let mut out = String::from(
        "Whole-world table (the play agent's own view; reference anything by the id in brackets):\n",
    );

    out.push_str("  Places:");
    if snapshot.places.is_empty() {
        out.push_str(" none");
    }
    for place in &snapshot.places {
        // Id beside label (PA.f74): a bare label list is ambiguous for two
        // subjects sharing one label.
        let occupants = joined(
            snapshot
                .subjects
                .iter()
                .filter(|subject| subject.position == Some(place.id))
                .map(|subject| format!("{} [{}]", subject.label, id_text(subject.id)))
                .collect(),
        );
        match place.container {
            Some(container) => out.push_str(&format!(
                " {} [{}] (in {}); occupants: {occupants};",
                place.label,
                id_text(place.id),
                place_label(container)
            )),
            None => out.push_str(&format!(
                " {} [{}]; occupants: {occupants};",
                place.label,
                id_text(place.id)
            )),
        }
    }

    out.push_str("\n  Routes:");
    if snapshot.routes.is_empty() {
        out.push_str(" none");
    }
    for route in &snapshot.routes {
        out.push_str(&format!(
            " {} [{}]: {} -> {}, {}, {};",
            route.label,
            id_text(route.id),
            place_label(route.from),
            place_label(route.to),
            render_access(&route.access),
            if route.open { "open" } else { "closed" }
        ));
    }

    out.push_str("\n  Subjects:");
    if snapshot.subjects.is_empty() {
        out.push_str(" none");
    }
    for subject in &snapshot.subjects {
        let granted = joined(
            snapshot
                .affordances
                .iter()
                .filter(|entry| subject.affordances.contains(&entry.id))
                .map(|entry| entry.entry.kind.0.clone())
                .collect(),
        );
        let holdings = joined(
            subject
                .components
                .holdings
                .iter()
                .map(|(resource, quantity)| {
                    let label = snapshot
                        .resources
                        .iter()
                        .find(|entry| entry.id == *resource)
                        .map_or_else(
                            || "an unnamed resource".to_owned(),
                            |entry| entry.label.clone(),
                        );
                    format!("{} of {label} [{}]", quantity.0, id_text(*resource))
                })
                .collect(),
        );
        let dependencies = joined(
            subject
                .dependencies
                .iter()
                .map(|target| render_dependency_target(snapshot, target))
                .collect(),
        );
        // PA.f74: a subject's own authority grants, the offices it holds and
        // grants, and the forums whose standing covers it, with ids and kind
        // names rather than nothing at all — `create_commitment`'s
        // `authorized`/`has_standing` checks and a restricted route's
        // `requires` all name an authority kind or a grievance kind that
        // only these rows can confirm the subject actually carries.
        let authority = joined(
            subject
                .components
                .authority
                .iter()
                .map(|grant| render_authority_grant(snapshot, grant))
                .collect(),
        );
        let offices_held = joined(
            subject
                .offices_held
                .iter()
                .map(|office| render_office(snapshot, office))
                .collect(),
        );
        let offices_granted = joined(
            subject
                .offices_granted
                .iter()
                .map(|office| render_office(snapshot, office))
                .collect(),
        );
        let forums = joined(
            subject
                .redress
                .iter()
                .map(|forum| render_forum(snapshot, forum))
                .collect(),
        );
        out.push_str(&format!(
            " {} [{}] ({}, {}, in {}, retired: {}, granted: {granted}, holdings: {holdings}, \
              depends on: {dependencies}, authority: {authority}, offices held: {offices_held}, \
              offices granted: {offices_granted}, forums: {forums});",
            subject.label,
            id_text(subject.id),
            render_subject_kind(subject.kind),
            subject
                .controller_mode
                .map_or_else(|| "external".to_owned(), |mode| render_controller_mode(mode).to_owned()),
            subject
                .position
                .map_or_else(|| "nowhere".to_owned(), place_label),
            subject.retired,
        ));
    }

    out.push_str("\n  Affordances:");
    if snapshot.affordances.is_empty() {
        out.push_str(" none");
    }
    for affordance in &snapshot.affordances {
        out.push_str(&format!(
            " {} [{}], roles: {}, {};",
            affordance.entry.kind.0,
            id_text(affordance.id),
            if affordance.entry.roles.is_empty() {
                "none".to_owned()
            } else {
                affordance
                    .entry
                    .roles
                    .iter()
                    .map(|role| role.role.0.clone())
                    .collect::<Vec<_>>()
                    .join("/")
            },
            if affordance.entry.carries_speech {
                "speech"
            } else {
                "silent"
            }
        ));
    }

    out.push_str("\n  Resources:");
    if snapshot.resources.is_empty() {
        out.push_str(" none");
    }
    for resource in &snapshot.resources {
        out.push_str(&format!(" {} [{}];", resource.label, id_text(resource.id)));
    }

    out.push_str("\n  Channels:");
    if snapshot.channels.is_empty() {
        out.push_str(" none");
    }
    for channel in &snapshot.channels {
        out.push_str(&format!(
            " [{}]: reaches {}, controller: {};",
            id_text(channel.id),
            render_reach(snapshot, &channel.reach),
            channel
                .controller
                .map_or_else(|| "none".to_owned(), |id| label_for_any_id(
                    snapshot,
                    &id_text(id)
                )
                .unwrap_or_else(|| format!("[{}]", id_text(id))))
        ));
    }

    // Omniscient: every fact the world holds, including one nobody currently
    // perceives and one whose standing is `ruled` — a fact known only through
    // a subject's own knowledge rows would miss both.
    out.push_str("\n  Facts:");
    if snapshot.facts.is_empty() {
        out.push_str(" none");
    }
    for fact in &snapshot.facts {
        // Id beside label (PA.f74): a bare label list is ambiguous for two
        // subjects sharing one label.
        let knowers = joined(
            fact.known_by
                .iter()
                .map(|id| {
                    snapshot
                        .subjects
                        .iter()
                        .find(|subject| subject.id == *id)
                        .map_or_else(
                            || format!("an unknown subject [{}]", id_text(*id)),
                            |subject| format!("{} [{}]", subject.label, id_text(*id)),
                        )
                })
                .collect(),
        );
        out.push_str(&format!(
            " \"{}\" [{}]: {}; known by: {knowers};",
            fact.statement.as_str(),
            id_text(fact.id),
            render_fact_standing(snapshot, &fact.standing)
        ));
    }

    out.push_str("\n  Commitments:");
    let mut any_commitment = false;
    for subject in &snapshot.subjects {
        for commitment in &subject.commitments {
            any_commitment = true;
            out.push_str(&format!(
                " {}/{} held by {} [{}]: {}{}, due {} minutes{}, past due: {}: \"{}\";",
                id_text(commitment.key.command),
                commitment.key.index,
                subject.label,
                id_text(subject.id),
                render_commitment_kind(commitment.kind),
                commitment.counterparty.map_or_else(String::new, |id| format!(
                    ", with {}",
                    label_for_any_id(snapshot, &id_text(id))
                        .unwrap_or_else(|| format!("[{}]", id_text(id)))
                )),
                commitment.due.0,
                commitment
                    .period
                    .map_or_else(String::new, |period| format!(", every {} minutes", period.minutes())),
                commitment.past_due,
                commitment.statement.as_str(),
            ));
        }
    }
    if !any_commitment {
        out.push_str(" none");
    }

    out.push_str("\n  Pressures:");
    let mut any_pressure = false;
    for subject in &snapshot.subjects {
        for pressure in &subject.pressures {
            any_pressure = true;
            out.push_str(&format!(
                " {} [{}]: {} from {};",
                subject.label,
                id_text(subject.id),
                pressure.magnitude.0,
                render_pressure_source(snapshot, &pressure.source)
            ));
        }
    }
    if !any_pressure {
        out.push_str(" none");
    }

    out.push_str("\n  Persona material:");
    let mut any_material = false;
    for subject in &snapshot.subjects {
        let Some(material) = &subject.material else {
            continue;
        };
        any_material = true;
        // PA.f74: values and memories are carried in their own authored
        // order — `joined`'s sort would scramble a voice's actual sequence,
        // which nothing about their content is meant to re-derive.
        let values: Vec<String> = material.values.iter().map(|value| value.as_str().to_owned()).collect();
        let values = if values.is_empty() { "none".to_owned() } else { values.join(", ") };
        let memories: Vec<String> = material
            .memories
            .iter()
            .map(|memory| memory.as_str().to_owned())
            .collect();
        let memories = if memories.is_empty() { "none".to_owned() } else { memories.join(", ") };
        let reads = joined(
            material
                .reads
                .iter()
                .map(|(id, statement)| {
                    let label = label_for_any_id(snapshot, &id_text(*id))
                        .unwrap_or_else(|| format!("[{}]", id_text(*id)));
                    format!("{label}: {}", statement.as_str())
                })
                .collect(),
        );
        out.push_str(&format!(
            " {} [{}]: voice \"{}\"; values: {values}; memories: {memories}; reads: {reads};",
            subject.label,
            id_text(subject.id),
            material.voice.as_str(),
        ));
    }
    if !any_material {
        out.push_str(" none");
    }

    out.push_str(&format!(
        "\n  Clock: the world reads {} fictional minutes.",
        snapshot.now.0
    ));
    out.push_str(&format!("\n  Brief: {}\n", snapshot.brief));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{Custody, Topology, activate, admit_custody, admit_topology, auth_principal, creation, owner, speech_world, submit_owner};

    /// The names Dungeon is expected to pin in Cut 8 (PA-Q7 B). The library
    /// itself hard-codes none of them; this list exists only to drive this
    /// test.
    // PA.f75: `communicate` was removed from `PLAY_TOOLS`; Dungeon drops it
    // from its own list too. This is now the complete 24-tool list.
    // PA.f88: this is the library's own test fixture, kept in sync by hand.
    // Dungeon's `PLAY_TOOLS` in `ghostlight-dungeon/src/play.rs` is the
    // single policy owner of what the play agent may actually call; this
    // copy pins nothing there and is not read by production code.
    const PLAY_TOOLS: &[&str] = &[
        "relocate",
        "transfer",
        "consume",
        "mint",
        "bind",
        "release",
        "witness",
        "acquire_knowledge",
        "forget",
        "create_commitment",
        "discharge_commitment",
        "advance_pressure",
        "reduce_pressure",
        "open_route",
        "close_route",
        "declare_place",
        "declare_route",
        "declare_resource",
        "declare_fact",
        "declare_subject",
        "set_persona_material",
        "retire",
        "grant_affordance",
        "revoke_affordance",
    ];

    /// A standing world holding one of every kind `PLAY_TOOLS` may take,
    /// composite fields included: the shared topology and custody fixtures,
    /// a claimed fact one subject knows, a ruled fact nobody knows (`Ruled`
    /// facts require `Play`, so this one is authored after `activate`), a
    /// declared channel, a commitment, a pressure row, and a dependency.
    /// `custody.ingot` is left unheld, so a resource id in the view can only
    /// have come from the Resources line itself, never a holding — the
    /// dependency instead targets `custody.counterparty`, a subject.
    struct PlayFixture {
        topology: Topology,
        custody: Custody,
        claimed_fact: EntityId,
        ruled_fact: EntityId,
        channel: EntityId,
        commitment_key: patch::CommitmentKey,
        snapshot: WorldSnapshot,
    }

    fn play_fixture() -> PlayFixture {
        let directory = tempfile::tempdir().expect("a temp dir");
        let mut kernel = crate::WorldKernel::create(
            directory.path().join("world.cc"),
            creation(super::CommandId::new(), "PlayTable"),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let topology = admit_topology(&mut kernel);
        let custody = admit_custody(&mut kernel, &topology);
        let before = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &before,
            crate::CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: vec![
                        Declaration::Fact(patch::FactDeclaration {
                            handle: patch::DraftHandle::new("claimed"),
                            label: "The Claim".into(),
                            statement: Statement::new("The gate stands open by decree.").unwrap(),
                            standing: FactStandingRef::Claimed {
                                by: patch::Ref::Existing(custody.holder),
                            },
                        }),
                        Declaration::Channel(patch::ChannelDeclaration {
                            handle: patch::DraftHandle::new("horn"),
                            label: "The Horn".into(),
                            reach: patch::ReachRef::Place(patch::Ref::Existing(topology.yard)),
                            controller: Some(patch::Ref::Existing(custody.holder)),
                        }),
                    ],
                    operations: vec![
                        ComponentOp::AcquireKnowledge {
                            subject: patch::Ref::Existing(custody.holder),
                            fact: patch::Ref::Draft(patch::DraftHandle::new("claimed")),
                            source: patch::AuthoredSource::Witnessed,
                            confidence: patch::Confidence::Certain,
                        },
                        ComponentOp::Bind {
                            subject: patch::Ref::Existing(custody.holder),
                            target: patch::DependencyRef::Subject(patch::Ref::Existing(
                                custody.counterparty,
                            )),
                        },
                        ComponentOp::CreateCommitment {
                            subject: patch::Ref::Existing(custody.holder),
                            counterparty: Some(patch::Ref::Existing(custody.counterparty)),
                            kind: patch::CommitmentKind::Obligation,
                            due: crate::FictionalMinutes(500),
                            period: None,
                            checks: Vec::new(),
                            statement: Statement::new("A promise both parties will read.").unwrap(),
                        },
                        ComponentOp::AdvancePressure {
                            source: patch::PressureSourceRef::Subject(patch::Ref::Existing(
                                custody.counterparty,
                            )),
                            target: patch::Ref::Existing(custody.holder),
                            by: patch::PressureMagnitude(3),
                        },
                    ],
                    evidence: Vec::new(),
                },
            },
        );
        let active = activate(&mut kernel);

        // `Ruled` requires `Play`, and `Play` may not declare in `Draft`
        // (`require_answer`), so this lands only after `activate`.
        let ruled_handle = patch::DraftHandle::new("ruled");
        kernel
            .submit(
                crate::tests::command(
                    &active,
                    super::CommandId::new(),
                    crate::CallerId::System(crate::SystemCapability::Play),
                    crate::CommandBody::AdmitPatch {
                        answers: None,
                        patch: WorldPatch {
                            declarations: vec![Declaration::Fact(patch::FactDeclaration {
                                handle: ruled_handle,
                                label: "The Ruling".into(),
                                statement: Statement::new("Stated by the table.").unwrap(),
                                standing: FactStandingRef::Ruled,
                            })],
                            operations: Vec::new(),
                            evidence: Vec::new(),
                        },
                    },
                ),
                &crate::AuthenticatedCaller::fixture(crate::CallerId::System(
                    crate::SystemCapability::Play,
                )),
            )
            .expect("the play authority may rule a fact in Active");

        let snapshot = kernel.snapshot().unwrap();
        let claimed_fact = snapshot
            .subjects
            .iter()
            .find(|subject| subject.id == custody.holder)
            .expect("the holder is in the snapshot")
            .knowledge
            .first()
            .expect("the holder was granted the claim")
            .fact;
        let ruled_fact = snapshot
            .facts
            .iter()
            .find(|fact| matches!(fact.standing, crate::FactStandingView::Ruled))
            .expect("the ruled fact is in the omniscient facts list")
            .id;
        let channel = snapshot
            .channels
            .first()
            .expect("the declared channel is in the snapshot")
            .id;
        let commitment_key = snapshot
            .subjects
            .iter()
            .find(|subject| subject.id == custody.holder)
            .expect("the holder is in the snapshot")
            .commitments
            .first()
            .expect("the holder holds the created commitment")
            .key;
        PlayFixture {
            topology,
            custody,
            claimed_fact,
            ruled_fact,
            channel,
            commitment_key,
            snapshot,
        }
    }

    fn fixture_id(
        referent: &str,
        topology: &Topology,
        custody: &Custody,
        fact: EntityId,
        snapshot: &WorldSnapshot,
    ) -> String {
        match referent {
            "subject" => id_text(custody.holder),
            "place" => id_text(topology.yard),
            "route" => id_text(topology.ramp),
            // Deliberately the unheld resource: see `play_fixture`.
            "resource" => id_text(custody.ingot),
            "fact" => id_text(fact),
            "affordance" => id_text(
                snapshot
                    .affordances
                    .first()
                    .expect("the fixture world declares an affordance")
                    .id,
            ),
            other => panic!("no fixture id for referent `{other}`"),
        }
    }

    fn existing_ref_value(id: &str) -> Value {
        serde_json::json!({"ref": "existing", "value": id})
    }

    /// Rule: a Dungeon-supplied name outside `PATCH_TOOLS` is refused, not
    /// silently dropped.
    #[test]
    fn authoring_tools_refuse_an_unknown_name() {
        let error = authoring_tools(&["mint", "not_a_tool"]).unwrap_err();
        assert_eq!(error, TableError::UnknownAuthoringTool("not_a_tool".to_owned()));
    }

    /// Mutation M7.2 (PA.f59: the doc here previously described the
    /// unknown-name refusal above, not this test): if `authoring_tools`
    /// silently dropped a known `PATCH_TOOLS` name instead of admitting it,
    /// `tools.len()` would fall short of `names.len()` and this assertion is
    /// the one that would catch it.
    #[test]
    fn authoring_tools_admits_every_known_name() {
        let names: Vec<&str> = PATCH_TOOLS.iter().map(|entry| entry.name).collect();
        let tools = authoring_tools(&names).expect("every PATCH_TOOLS name is a known name");
        assert_eq!(tools.len(), names.len());
    }

    /// Rule: every non-session authoring tool's own generated example decodes
    /// to exactly one patch item, through `decode_authoring_call`.
    #[test]
    fn every_authoring_tool_decodes_its_own_example() {
        for entry in PATCH_TOOLS {
            if matches!(entry.shape, PatchToolShape::Session) {
                continue;
            }
            let mut object = serde_json::Map::new();
            for field in entry.fields {
                object.insert(field.name.to_owned(), patch::field_example(field.kind));
            }
            let arguments = Value::Object(object).to_string();
            let decoded = decode_authoring_call(entry.name, &arguments)
                .unwrap_or_else(|error| panic!("{} did not decode: {error}", entry.name));
            assert_eq!(
                decoded.declarations.len() + decoded.operations.len(),
                1,
                "{} decoded to something other than one item",
                entry.name
            );
        }
    }

    /// Rule (PA.f54): a later call in one run resolves a draft handle an
    /// earlier call in the same run declared, because every call's item lands
    /// in the run's one shared `WorldPatch`. `declare_place` has no field a
    /// `relocate` call takes (it takes a subject and a route, not a place),
    /// so `declare_route`'s `from` is the real field this catalog offers that
    /// takes a place reference; it stands for the same claim PA.f54 names.
    #[test]
    fn decode_authoring_calls_shares_draft_handles_across_one_run() {
        let place = serde_json::json!({
            "handle": "yard",
            "label": "The Cavity Yard",
            "container": null,
        })
        .to_string();
        let route = serde_json::json!({
            "handle": "ramp",
            "label": "The Yard Ramp",
            "from": {"ref": "draft", "value": "yard"},
            "to": {"ref": "existing", "value": "11111111-1111-4111-8111-111111111111"},
            "access": {"access": "public"},
            "cost": 1,
        })
        .to_string();
        let batch = decode_authoring_calls(&[("declare_place", &place), ("declare_route", &route)])
            .expect("both calls in the run decode together");
        assert_eq!(batch.patch.declarations.len(), 2);
        assert_eq!(batch.patch.operations.len(), 0);
        let Declaration::Route(declared) = &batch.patch.declarations[1] else {
            panic!("declare_route decoded to something other than a route declaration");
        };
        assert_eq!(declared.from, patch::Ref::Draft(patch::DraftHandle::new("yard")));

        // The site map: each declaration's index names the call index that
        // produced it.
        assert_eq!(
            batch.site_calls.get(&PatchSite::Declaration(0)),
            Some(&0),
            "declare_place's declaration must be attributed to call #0"
        );
        assert_eq!(
            batch.site_calls.get(&PatchSite::Declaration(1)),
            Some(&1),
            "declare_route's declaration must be attributed to call #1"
        );
    }

    /// Rule (PA.f54): a decode error names the failing call's index, not only
    /// its tool name — the same name can appear more than once in one run.
    #[test]
    fn decode_authoring_calls_names_the_failing_call_index() {
        let good = serde_json::json!({
            "handle": "yard",
            "label": "The Cavity Yard",
            "container": null,
        })
        .to_string();
        let error = decode_authoring_calls(&[
            ("declare_place", &good),
            ("declare_place", "not json at all"),
        ])
        .unwrap_err();
        assert!(error.contains("call #1"), "{error}");
        assert!(error.contains("declare_place"), "{error}");
    }

    /// Rule (PA.f59): the elaborator's own decode and the table's decode
    /// agree over the *whole* catalog, not just `open_route` — for every
    /// non-session `PATCH_TOOLS` entry's own generated example, the
    /// elaboration path (`apply_tool_call`) and the table path
    /// (`decode_authoring_call`) produce the whole same `WorldPatch`,
    /// because the elaborator's arm calls the table's function rather than
    /// carrying a second copy.
    #[test]
    fn the_elaborator_and_the_table_agree_over_every_tool() {
        for entry in PATCH_TOOLS {
            if matches!(entry.shape, PatchToolShape::Session) {
                continue;
            }
            let mut object = serde_json::Map::new();
            for field in entry.fields {
                object.insert(field.name.to_owned(), patch::field_example(field.kind));
            }
            let arguments = Value::Object(object).to_string();
            let mut draft = WorldPatch::default();
            let mut gaps = Vec::new();
            let mut submitted = false;
            crate::elaboration::apply_tool_call(
                entry.name,
                &arguments,
                &mut draft,
                &mut gaps,
                &mut submitted,
            );
            let table_patch = decode_authoring_call(entry.name, &arguments)
                .unwrap_or_else(|error| panic!("{} did not decode through the table: {error}", entry.name));
            assert!(gaps.is_empty(), "{}: the elaborator recorded a gap: {gaps:?}", entry.name);
            assert_eq!(
                draft.declarations, table_patch.declarations,
                "{}: the elaborator's and the table's declarations disagree",
                entry.name
            );
            assert_eq!(
                draft.operations, table_patch.operations,
                "{}: the elaborator's and the table's operations disagree",
                entry.name
            );
            assert_eq!(
                draft.evidence, table_patch.evidence,
                "{}: the elaborator's and the table's evidence disagree",
                entry.name
            );
        }
    }

    /// Rule: an actor tool call decodes against the subject's own currently
    /// issued opportunity, never a value a caller could forge (compare
    /// PA.f41: `PersonaLane` refused a forged `affordance_ids` the same way).
    #[test]
    fn an_actor_tool_decodes_to_the_subjects_current_opportunity() {
        let directory = tempfile::tempdir().expect("a temp dir");
        let mut kernel = crate::WorldKernel::create(
            directory.path().join("world.cc"),
            creation(super::CommandId::new(), "PlayActor"),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let (speech, active) = speech_world(&mut kernel);
        let expected = active
            .opportunities
            .iter()
            .find(|opportunity| opportunity.scope.subject_id == speech.speaker)
            .cloned()
            .expect("the speaker holds a live opportunity");
        let arguments = serde_json::json!({
            "target": id_text(speech.listener),
            "text": "a whispered warning",
        })
        .to_string();
        let (opportunity, invocation) =
            decode_actor_call(&active, speech.speaker, "", "whisper", &arguments)
                .expect("whisper decodes against the speaker's own opportunity");
        assert_eq!(opportunity, expected);
        assert_eq!(
            invocation.speech.as_ref().map(Statement::as_str),
            Some("a whispered warning")
        );
    }

    /// Rule (PA.f55): `actor_tools` never offers `record_need` or
    /// `finish_without_proposal` — those end a turn, which is the controller
    /// lane's own vocabulary, and `decode_actor_call` has no arm for either.
    /// Every name it does emit decodes as emitted once the same prefix is
    /// handed to `decode_actor_call`.
    #[test]
    fn actor_tools_offers_only_names_decode_actor_call_accepts() {
        let directory = tempfile::tempdir().expect("a temp dir");
        let mut kernel = crate::WorldKernel::create(
            directory.path().join("world.cc"),
            creation(super::CommandId::new(), "PlayActorVocabulary"),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let (speech, active) = speech_world(&mut kernel);
        let prefix = "c0__";
        let tools = actor_tools(prefix, &active, speech.speaker);
        assert!(!tools.is_empty(), "the speaker holds granted affordances");
        for tool in &tools {
            assert!(
                !tool.name.ends_with("record_need") && !tool.name.ends_with("finish_without_proposal"),
                "actor_tools must not offer the controller lane's turn-enders: {}",
                tool.name
            );
        }
        let arguments = serde_json::json!({
            "target": id_text(speech.listener),
            "text": "a whispered warning",
        })
        .to_string();
        let (_, invocation) =
            decode_actor_call(&active, speech.speaker, prefix, &format!("{prefix}whisper"), &arguments)
                .expect("a name actor_tools emits decodes as emitted, prefix included");
        assert_eq!(
            invocation.speech.as_ref().map(Statement::as_str),
            Some("a whispered warning")
        );
    }

    /// Rule (PA.f58): a non-string `display` is refused, not silently
    /// dropped.
    #[test]
    fn decode_actor_call_refuses_a_non_string_display() {
        let directory = tempfile::tempdir().expect("a temp dir");
        let mut kernel = crate::WorldKernel::create(
            directory.path().join("world.cc"),
            creation(super::CommandId::new(), "PlayActorDisplay"),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let (speech, active) = speech_world(&mut kernel);
        let arguments = serde_json::json!({
            "target": id_text(speech.listener),
            "text": "a whispered warning",
            "display": 7,
        })
        .to_string();
        let error = decode_actor_call(&active, speech.speaker, "", "whisper", &arguments).unwrap_err();
        assert!(error.contains("display"), "{error}");
    }

    /// Rule (PA.f58): `text` on an entry that carries no speech is refused,
    /// not silently dropped.
    #[test]
    fn decode_actor_call_refuses_text_on_a_silent_entry() {
        let fixture = play_fixture();
        let holder = fixture
            .snapshot
            .subjects
            .iter()
            .find(|subject| subject.id == fixture.custody.holder)
            .expect("the holder is in the snapshot");
        let carry = fixture
            .snapshot
            .affordances
            .iter()
            .find(|entry| holder.affordances.contains(&entry.id) && entry.entry.kind.0 == "carry")
            .expect("the holder holds the silent `carry` affordance");
        let mut object = serde_json::Map::new();
        for role in &carry.entry.roles {
            object.insert(role.role.0.clone(), Value::String(id_text(fixture.custody.holder)));
        }
        object.insert("slot_0_qty".into(), Value::from(1));
        object.insert("text".into(), Value::String("this affordance carries no speech".into()));
        let arguments = Value::Object(object).to_string();
        let error = decode_actor_call(&fixture.snapshot, fixture.custody.holder, "", "carry", &arguments)
            .unwrap_err();
        assert!(error.contains("text"), "{error}");
    }

    /// Rule (PA.f71): a non-empty prefix must actually be carried by the
    /// name. The bare `kind` here (`whisper`, unprefixed) is a granted
    /// entry's own name, so the old fallback to `kind` on a failed strip
    /// would let it decode anyway — a name `actor_tools` never emits under
    /// this prefix must still be refused.
    #[test]
    fn decode_actor_call_refuses_a_name_without_the_actor_prefix() {
        let directory = tempfile::tempdir().expect("a temp dir");
        let mut kernel = crate::WorldKernel::create(
            directory.path().join("world.cc"),
            creation(super::CommandId::new(), "PlayActorPrefix"),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let (speech, active) = speech_world(&mut kernel);
        let arguments = serde_json::json!({
            "target": id_text(speech.listener),
            "text": "a whispered warning",
        })
        .to_string();
        let error = decode_actor_call(&active, speech.speaker, "c0__", "whisper", &arguments).unwrap_err();
        assert!(error.contains("whisper"), "{error}");
        assert!(error.contains("c0__"), "{error}");
    }

    /// Mutation M7.1: if `decode_actor_call` used the snapshot's first
    /// opportunity instead of the subject's own, this would no longer hold —
    /// `speech.listener` holds a different, later-ordered opportunity.
    #[test]
    fn an_actor_tool_never_borrows_another_subjects_opportunity() {
        let directory = tempfile::tempdir().expect("a temp dir");
        let mut kernel = crate::WorldKernel::create(
            directory.path().join("world.cc"),
            creation(super::CommandId::new(), "PlayActorGuard"),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let (speech, active) = speech_world(&mut kernel);
        let expected = active
            .opportunities
            .iter()
            .find(|opportunity| opportunity.scope.subject_id == speech.listener)
            .cloned()
            .expect("the listener holds a live opportunity");
        let arguments = serde_json::json!({
            "target": id_text(speech.speaker),
            "text": "a returned whisper",
        })
        .to_string();
        let (opportunity, _) = decode_actor_call(&active, speech.listener, "", "whisper", &arguments)
            .expect("whisper decodes against the listener's own opportunity");
        assert_eq!(opportunity, expected);
        assert_ne!(
            opportunity.scope.subject_id,
            speech.speaker,
            "the listener's call must never resolve to the speaker's opportunity"
        );
    }

    /// Rule: `describe_refusal` names the failed precondition rather than
    /// dumping the mismatch's raw shape.
    #[test]
    fn describe_refusal_names_the_failed_precondition() {
        let snapshot = play_fixture().snapshot;
        let error = KernelError::ActionRejected(vec![ActionMismatch::ActorNotPresent {
            precondition: 2,
        }]);
        let text = describe_refusal(&snapshot, None, None, None, &error);
        assert!(text.contains("precondition #2"), "{text}");
        assert!(text.contains("not present"), "{text}");
    }

    /// Every other `KernelError` renders by its own `Display`.
    #[test]
    fn describe_refusal_falls_back_to_display_for_every_other_error() {
        let snapshot = play_fixture().snapshot;
        let error = KernelError::Unauthorized;
        assert_eq!(describe_refusal(&snapshot, None, None, None, &error), error.to_string());
    }

    /// Rule (PA.f56): `WrongKind` names both the expected and the actual
    /// kind, in plain words, not Rust `Debug`.
    #[test]
    fn describe_refusal_names_wrong_kinds_expected_and_actual() {
        let fixture = play_fixture();
        let mismatch = Mismatch::WrongKind {
            site: patch::Site::Operation(3),
            referent: patch::RefName::Subject(patch::Ref::Existing(fixture.custody.holder)),
            expected: RefKind::Entity(crate::EntityKind::Place),
            actual: RefKind::Subject(None),
        };
        let text = describe_refusal(
            &fixture.snapshot,
            None,
            None,
            None,
            &KernelError::PatchRejected(vec![mismatch]),
        );
        assert!(text.contains("place"), "{text}");
        assert!(text.contains("subject"), "{text}");
        assert!(!text.contains("RefKind") && !text.contains("Entity("), "{text}");
    }

    /// Rule (PA.f56): `CustodyNotConserved` names the resource whose ledger
    /// failed to balance.
    #[test]
    fn describe_refusal_names_custody_not_conserveds_resource() {
        let fixture = play_fixture();
        let mismatch = Mismatch::CustodyNotConserved {
            resource: patch::RefName::Entity(patch::Ref::Existing(fixture.custody.ingot)),
        };
        let text = describe_refusal(
            &fixture.snapshot,
            None,
            None,
            None,
            &KernelError::PatchRejected(vec![mismatch]),
        );
        let ingot = fixture
            .snapshot
            .resources
            .iter()
            .find(|resource| resource.id == fixture.custody.ingot)
            .expect("the ingot is a declared resource");
        assert!(text.contains(&ingot.label), "{text}");
    }

    /// Rule (PA.f56): `EmptyEvidence`'s `position` is an evidence-list index,
    /// never rendered as a declaration site the way `EmptyHandle`'s own
    /// `position` is — the two must not read as the same thing.
    #[test]
    fn describe_refusal_never_conflates_empty_evidence_with_a_declaration_site() {
        let fixture = play_fixture();
        let text = describe_refusal(
            &fixture.snapshot,
            None,
            None,
            None,
            &KernelError::PatchRejected(vec![Mismatch::EmptyEvidence { position: 4 }]),
        );
        assert!(text.contains("evidence"), "{text}");
        assert!(!text.contains("declaration"), "{text}");
    }

    /// Rule (PA.f56): when the caller has the `DecodedBatch` that produced
    /// the rejected patch, a declaration or operation site names the exact
    /// call index `batch`'s own site map attributes it to, not the raw
    /// index into the patch's own declarations or operations.
    #[test]
    fn describe_refusal_names_the_exact_call_through_a_batch_map() {
        let place = serde_json::json!({"handle": "yard", "label": "The Cavity Yard", "container": null})
            .to_string();
        let another =
            serde_json::json!({"handle": "hall", "label": "The Long Hall", "container": null})
                .to_string();
        let batch = decode_authoring_calls(&[("declare_place", &place), ("declare_place", &another)])
            .expect("both calls decode");
        let fixture = play_fixture();
        let text = describe_refusal(
            &fixture.snapshot,
            None,
            None,
            Some(&batch),
            &KernelError::PatchRejected(vec![Mismatch::EmptyHandle { position: 1 }]),
        );
        assert!(text.starts_with("call #1:"), "{text}");
        assert!(!text.contains("tool call"), "{text}");
    }

    /// Rule (PA.f73): `UnresolvedDraft` names the handle that never resolved
    /// and the kind it was expected to resolve to, not just its site.
    /// PA.f79: the handle used to be `missing_place`, which contains the
    /// expected-kind word itself — a mutation that replaced the rendered
    /// kind with a fixed `"thing"` still passed `text.contains("place")`.
    /// The handle here carries no kind word, and the kind is pinned exactly.
    #[test]
    fn describe_refusal_names_unresolved_drafts_handle_and_expected_kind() {
        let fixture = play_fixture();
        let mismatch = Mismatch::UnresolvedDraft {
            site: patch::Site::Operation(3),
            referent: patch::DraftHandle::new("unresolved_target"),
            expected: RefKind::Entity(crate::EntityKind::Place),
        };
        let text = describe_refusal(
            &fixture.snapshot,
            None,
            None,
            None,
            &KernelError::PatchRejected(vec![mismatch]),
        );
        assert!(text.contains("unresolved_target"), "{text}");
        assert!(
            text.contains("draft handle `unresolved_target` was never resolved to a place"),
            "{text}"
        );
    }

    /// Rule (PA.f73): a handle declared twice in one run is named by both
    /// calls that declared it, not by one ambiguous "the call that declared
    /// `handle`" — the old generic wording could only ever pick one.
    /// PA.f79: with only the two colliding declarations in the batch, a
    /// filter that matched every declaration regardless of its handle would
    /// still produce "call #0 and call #1" and pass. A third, unrelated
    /// declaration pins the filter: it must be named by neither call.
    #[test]
    fn describe_refusal_names_both_calls_for_a_duplicate_handle() {
        let first = serde_json::json!({"handle": "yard", "label": "The Cavity Yard", "container": null})
            .to_string();
        let second = serde_json::json!({"handle": "yard", "label": "The Second Yard", "container": null})
            .to_string();
        let unrelated =
            serde_json::json!({"handle": "gate", "label": "The Rain Gate", "container": null})
                .to_string();
        let batch = decode_authoring_calls(&[
            ("declare_place", &first),
            ("declare_place", &second),
            ("declare_place", &unrelated),
        ])
        .expect("all three calls decode; duplicate handles are a commit-time, not decode-time, refusal");
        let fixture = play_fixture();
        let text = describe_refusal(
            &fixture.snapshot,
            None,
            None,
            Some(&batch),
            &KernelError::PatchRejected(vec![Mismatch::DuplicateHandle {
                handle: patch::DraftHandle::new("yard"),
            }]),
        );
        assert!(text.contains("call #0"), "{text}");
        assert!(text.contains("call #1"), "{text}");
        assert!(!text.contains("call #2"), "{text}");
    }

    /// Rule (PA.f73): an `EmptyEvidence` mismatch names the exact call that
    /// cited the empty or duplicate evidence reference, through
    /// `DecodedBatch::evidence_calls`, the same way a declaration or
    /// operation site names its call through `site_calls`.
    #[test]
    fn describe_refusal_maps_empty_evidence_to_its_call() {
        // PA.f75 (mutation M3): a single-call batch cannot distinguish "maps
        // to the exact call" from "always maps to call 0" — call 0 is right
        // either way. `admit` (the only evidence-carrying call) goes second,
        // behind a `declare_place` that carries none, so the true call index
        // for its evidence entry is 1.
        let place = serde_json::json!({"handle": "yard", "label": "The Cavity Yard", "container": null})
            .to_string();
        let admit = PATCH_TOOLS
            .iter()
            .find(|entry| entry.name == "admit")
            .expect("admit is a patch tool");
        let mut object = serde_json::Map::new();
        for field in admit.fields {
            object.insert(field.name.to_owned(), patch::field_example(field.kind));
        }
        let admit_arguments = Value::Object(object).to_string();
        let batch = decode_authoring_calls(&[("declare_place", &place), ("admit", &admit_arguments)])
            .expect("both calls decode");
        assert_eq!(
            batch.patch.evidence.len(),
            1,
            "admit's own example carries exactly one evidence entry"
        );
        assert_eq!(
            batch.evidence_calls,
            vec![1],
            "the evidence entry must be attributed to call #1, not call #0"
        );
        let fixture = play_fixture();
        let text = describe_refusal(
            &fixture.snapshot,
            None,
            None,
            Some(&batch),
            &KernelError::PatchRejected(vec![Mismatch::EmptyEvidence { position: 0 }]),
        );
        assert!(text.starts_with("call #1:"), "{text}");
    }

    /// Rule (PA.f59, mutation X6): a declaration site and an operation site
    /// never trade wording. `EmptyHandle`'s `position` names a declaration;
    /// `SubjectNotAtOrigin`'s `operation` names an operation; with no batch
    /// map in hand, each must keep its own word ("declaration"/"operation"),
    /// never the other's.
    #[test]
    fn describe_refusal_never_swaps_declaration_and_operation_site_wording() {
        let fixture = play_fixture();
        let declaration_text = describe_refusal(
            &fixture.snapshot,
            None,
            None,
            None,
            &KernelError::PatchRejected(vec![Mismatch::EmptyHandle { position: 3 }]),
        );
        assert!(
            declaration_text.starts_with("declaration tool call #3:"),
            "{declaration_text}"
        );
        let operation_text = describe_refusal(
            &fixture.snapshot,
            None,
            None,
            None,
            &KernelError::PatchRejected(vec![Mismatch::SubjectNotAtOrigin { operation: 3 }]),
        );
        assert!(
            operation_text.starts_with("operation tool call #3:"),
            "{operation_text}"
        );
    }

    /// Rule (PA.f56): `ActionRejected` renders the failed precondition's own
    /// role and what the actor's invocation actually bound it to, when
    /// `entry` and `invocation` are given.
    #[test]
    fn describe_refusal_renders_a_preconditions_bound_role() {
        let fixture = play_fixture();
        let holder = fixture
            .snapshot
            .subjects
            .iter()
            .find(|subject| subject.id == fixture.custody.holder)
            .expect("the holder is in the snapshot");
        let carry = fixture
            .snapshot
            .affordances
            .iter()
            .find(|entry| holder.affordances.contains(&entry.id) && entry.entry.kind.0 == "carry")
            .expect("the holder holds `carry`");
        let mut object = serde_json::Map::new();
        for role in &carry.entry.roles {
            object.insert(role.role.0.clone(), Value::String(id_text(fixture.custody.holder)));
        }
        object.insert("slot_0_qty".into(), Value::from(1));
        let arguments = Value::Object(object).to_string();
        let (_, invocation) =
            decode_actor_call(&fixture.snapshot, fixture.custody.holder, "", "carry", &arguments)
                .expect("carry decodes against the holder's own opportunity");
        // Precondition #0 on `carry` is `Present { at: place_role() }`.
        let error = KernelError::ActionRejected(vec![ActionMismatch::ActorNotPresent { precondition: 0 }]);
        let text = describe_refusal(&fixture.snapshot, Some(carry), Some(&invocation), None, &error);
        assert!(text.contains("role `place`"), "{text}");
        assert!(text.contains(&holder.label), "{text}");
    }

    /// Rule (PA.f57): `describe_refusal_to_actor` reveals the failed
    /// precondition's role name and nothing else — never a resolved label,
    /// an id, or a fact's statement. A precondition over a fact the actor
    /// does not hold must not leak that fact's identity back to it.
    #[test]
    fn describe_refusal_to_actor_reveals_only_the_bound_role_name() {
        let fixture = play_fixture();
        let holder = fixture
            .snapshot
            .subjects
            .iter()
            .find(|subject| subject.id == fixture.custody.holder)
            .expect("the holder is in the snapshot");
        let carry = fixture
            .snapshot
            .affordances
            .iter()
            .find(|entry| holder.affordances.contains(&entry.id) && entry.entry.kind.0 == "carry")
            .expect("the holder holds `carry`");
        let error = KernelError::ActionRejected(vec![ActionMismatch::InsufficientHolding { precondition: 1 }]);
        let text = describe_refusal_to_actor(carry, &error);
        assert!(text.contains("`resource`"), "{text}");
        assert!(!text.contains(&id_text(fixture.custody.holder)), "{text}");
        assert!(!text.contains(&holder.label), "{text}");
    }

    /// A minimal `AffordanceSnapshot` carrying exactly these preconditions
    /// and nothing else, for tests that probe `precondition_roles` and the
    /// two refusal renderers directly rather than through a live kernel.
    fn affordance_snapshot(preconditions: Vec<patch::Precondition>) -> AffordanceSnapshot {
        AffordanceSnapshot {
            id: crate::AffordanceId::issue(),
            entry: patch::Affordance {
                kind: patch::AffordanceKindName("probe".to_owned()),
                roles: Vec::new(),
                preconditions,
                effect_slots: Vec::new(),
                outcome_bands: Vec::new(),
                carries_speech: false,
            },
        }
    }

    /// Rule (PA.f72): `precondition_roles` reads the channel role a
    /// `CanBroadcast { via: Channel(role) }` binds, rather than dropping it.
    #[test]
    fn precondition_roles_reads_the_broadcast_channel_role() {
        let role = patch::Role("horn".into());
        let precondition = patch::Precondition::CanBroadcast {
            via: patch::AudienceSpec::Channel(role.clone()),
        };
        assert_eq!(precondition_roles(&precondition), vec![role]);
    }

    /// Rule (PA.f72): `precondition_roles` reads `CanReach`'s own `subject`
    /// role and the channel role its `via` binds, in that order.
    #[test]
    fn precondition_roles_reads_the_reach_channel_role_alongside_the_subject_role() {
        let subject_role = patch::Role("target".into());
        let channel_role = patch::Role("horn".into());
        let precondition = patch::Precondition::CanReach {
            subject: subject_role.clone(),
            via: patch::AudienceSpec::Channel(channel_role.clone()),
        };
        assert_eq!(precondition_roles(&precondition), vec![subject_role, channel_role]);
    }

    /// Rule (PA.f72): a `Colocated` audience still names no role, for both
    /// `CanBroadcast` and `CanReach` — only `Channel` names one.
    #[test]
    fn precondition_roles_names_no_role_for_a_colocated_audience() {
        assert!(precondition_roles(&patch::Precondition::CanBroadcast {
            via: patch::AudienceSpec::Colocated,
        })
        .is_empty());
        assert_eq!(
            precondition_roles(&patch::Precondition::CanReach {
                subject: patch::Role("target".into()),
                via: patch::AudienceSpec::Colocated,
            }),
            vec![patch::Role("target".into())]
        );
    }

    /// Rule (PA.f72): `describe_refusal_to_actor` uses plain wording for a
    /// precondition that names no role — `HasStanding` (`NoStanding`) — never
    /// the garbled "a precondition over role a precondition with no role,
    /// which you bound yourself" the old text produced.
    #[test]
    fn describe_refusal_to_actor_uses_plain_wording_for_no_standing() {
        let entry = affordance_snapshot(vec![patch::Precondition::HasStanding {
            grievance: patch::GrievanceKindName("noise".into()),
        }]);
        let error = KernelError::ActionRejected(vec![ActionMismatch::NoStanding { precondition: 0 }]);
        let text = describe_refusal_to_actor(&entry, &error);
        assert!(!text.contains("a precondition over role"), "{text}");
        assert!(text.contains("no role"), "{text}");
    }

    /// Rule (PA.f72): the same plain wording applies to `NoAudience` over a
    /// `Colocated` audience, which also names no role.
    #[test]
    fn describe_refusal_to_actor_uses_plain_wording_for_no_audience_colocated() {
        let entry = affordance_snapshot(vec![patch::Precondition::CanBroadcast {
            via: patch::AudienceSpec::Colocated,
        }]);
        let error = KernelError::ActionRejected(vec![ActionMismatch::NoAudience { precondition: 0 }]);
        let text = describe_refusal_to_actor(&entry, &error);
        assert!(!text.contains("a precondition over role"), "{text}");
        assert!(text.contains("no role"), "{text}");
    }

    /// Rule (PA.f72): an out-of-range precondition index also falls back to
    /// the plain wording rather than the garbled sentence.
    #[test]
    fn describe_refusal_to_actor_uses_plain_wording_for_an_out_of_range_precondition() {
        let entry = affordance_snapshot(Vec::new());
        let error = KernelError::ActionRejected(vec![ActionMismatch::ActorNotPresent { precondition: 9 }]);
        let text = describe_refusal_to_actor(&entry, &error);
        assert!(!text.contains("a precondition over role"), "{text}");
    }

    /// Rule (PA.f72): `describe_refusal_to_actor` reveals the channel role a
    /// failed `CanBroadcast` names, now that `precondition_roles` reads it.
    #[test]
    fn describe_refusal_to_actor_reveals_the_broadcast_channel_role() {
        let entry = affordance_snapshot(vec![patch::Precondition::CanBroadcast {
            via: patch::AudienceSpec::Channel(patch::Role("horn".into())),
        }]);
        let error = KernelError::ActionRejected(vec![ActionMismatch::NoAudience { precondition: 0 }]);
        let text = describe_refusal_to_actor(&entry, &error);
        assert!(text.contains("`horn`"), "{text}");
    }

    /// PA.f81: nothing pinned `describe_refusal_to_actor`'s exact wording for
    /// a role-bearing precondition — every prior test used `contains`, so
    /// appending a `{:?}` dump of the failed precondition (which would leak
    /// internal shape the actor never handed back) survived every one of
    /// them. Pin the full string for one role-bearing failure.
    #[test]
    fn describe_refusal_to_actor_exact_wording_for_a_role_bearing_precondition() {
        let entry = affordance_snapshot(vec![patch::Precondition::CanBroadcast {
            via: patch::AudienceSpec::Channel(patch::Role("horn".into())),
        }]);
        let error = KernelError::ActionRejected(vec![ActionMismatch::NoAudience { precondition: 0 }]);
        let text = describe_refusal_to_actor(&entry, &error);
        assert_eq!(text, "a precondition over role `horn`, which you bound yourself, failed");
    }

    /// PA.f81: the same pin for a roleless precondition failure
    /// (`HasStanding`/`NoStanding`).
    #[test]
    fn describe_refusal_to_actor_exact_wording_for_a_roleless_precondition() {
        let entry = affordance_snapshot(vec![patch::Precondition::HasStanding {
            grievance: patch::GrievanceKindName("noise".into()),
        }]);
        let error = KernelError::ActionRejected(vec![ActionMismatch::NoStanding { precondition: 0 }]);
        let text = describe_refusal_to_actor(&entry, &error);
        assert_eq!(text, "a precondition you bound no role for failed");
    }

    /// Rule (PA.f57): every `KernelError` other than `ActionRejected` —
    /// `MissingApprovals`, a store or journal error — reveals nothing to the
    /// actor.
    #[test]
    fn describe_refusal_to_actor_reveals_nothing_for_every_other_error() {
        let fixture = play_fixture();
        let holder = fixture
            .snapshot
            .subjects
            .iter()
            .find(|subject| subject.id == fixture.custody.holder)
            .expect("the holder is in the snapshot");
        let carry = fixture
            .snapshot
            .affordances
            .iter()
            .find(|entry| holder.affordances.contains(&entry.id) && entry.entry.kind.0 == "carry")
            .expect("the holder holds `carry`");
        // PA.f75: every `KernelError` variant but `ActionRejected` — all 31
        // named in `lib.rs`, none skipped — driven through
        // `describe_refusal_to_actor` (mutation M4: the actor form falling
        // back to `Display` for one of these, `PatchRejected` most of all,
        // would dump `Mismatch`/`Debug` internals; a store or journal detail,
        // a principal id, or a fact never belongs on this surface either).
        let principal = crate::PrincipalId::new("a principal id nobody outside the kernel should see");
        let scope = crate::DecisionScope {
            subject_id: crate::SubjectId::issue(),
        };
        for error in [
            KernelError::InvalidCommandId,
            KernelError::EmptyTitle,
            KernelError::EmptyPrincipal,
            KernelError::PatchRejected(vec![Mismatch::EmptyHandle { position: 0 }]),
            KernelError::WorldMismatch,
            KernelError::AuthenticationMismatch,
            KernelError::Unauthorized,
            KernelError::WrongPhase {
                expected: crate::WorldPhase::Active,
                actual: crate::WorldPhase::Draft,
            },
            KernelError::NotDraftApprover,
            KernelError::DraftAlreadyApproved,
            KernelError::MissingApprovals(vec![principal]),
            KernelError::OpportunityMismatch,
            KernelError::ScopeChanged {
                scope,
                expected: crate::ScopeDigest::fixture("expected digest nobody outside the kernel should see"),
                actual: crate::ScopeDigest::fixture("actual digest nobody outside the kernel should see"),
            },
            KernelError::ControllerMismatch,
            KernelError::AffordanceDenied,
            KernelError::RevisionMismatch { expected: 3, actual: 7 },
            KernelError::CommandIdConflict,
            KernelError::CreationConflict,
            KernelError::CreationTargetOccupied,
            KernelError::WorldNotCreated,
            KernelError::OpenedWorldMismatch,
            KernelError::RecoveryRequired {
                command_id: super::CommandId::new(),
            },
            KernelError::OwnershipLost,
            KernelError::Serialization("a serialization detail nobody outside the kernel should see".into()),
            KernelError::Store("a store detail nobody outside the kernel should see".into()),
            KernelError::CorruptJournal("a journal detail nobody outside the kernel should see".into()),
            KernelError::Invariant("an invariant detail nobody outside the kernel should see".into()),
            KernelError::AnswerRequired,
            KernelError::AnswerNotDerived,
            KernelError::AnswerNotSatisfied,
        ] {
            let text = describe_refusal_to_actor(carry, &error);
            assert_eq!(text, "refused", "{error:?} leaked detail to the actor: {text}");
        }
    }

    /// Rule (PA.f57): a failed precondition over a fact the actor does not
    /// hold reveals only the bare role name — never that fact's statement,
    /// its id, or its label — even when a real fact with all three sits
    /// right there in the snapshot the caller could have (but is not) handed.
    #[test]
    fn describe_refusal_to_actor_never_reveals_an_unknown_facts_identity() {
        let fixture = play_fixture();
        let holder = fixture
            .snapshot
            .subjects
            .iter()
            .find(|subject| subject.id == fixture.custody.holder)
            .expect("the holder is in the snapshot");
        let fact = fixture
            .snapshot
            .facts
            .iter()
            .find(|fact| fact.id == fixture.claimed_fact)
            .expect("the claimed fact is in the snapshot");
        let entry = affordance_snapshot(vec![patch::Precondition::Knows {
            fact: patch::Role("secret".into()),
            at_least: patch::Confidence::Certain,
        }]);
        let error = KernelError::ActionRejected(vec![ActionMismatch::FactUnknown { precondition: 0 }]);
        let text = describe_refusal_to_actor(&entry, &error);
        assert!(text.contains("`secret`"), "{text}");
        assert!(!text.contains(fact.statement.as_str()), "{text}");
        assert!(!text.contains(&id_text(fixture.claimed_fact)), "{text}");
        assert!(!text.contains(&holder.label), "{text}");
    }

    /// Rule: `table_view` prints every id a `PLAY_TOOLS` example takes,
    /// composite fields included — a commitment key (`discharge_commitment`),
    /// a pressure source (`advance_pressure`/`reduce_pressure`), a dependency
    /// target (`bind`/`release`), and a channel-audience id (`communicate`),
    /// not only the plain `Reference`/`OptionalReference`/`ReferenceSet`
    /// fields the pre-PA.f53 test covered. Mutation M7.3: printing resources
    /// by label alone, as the moved `render_world_structure` still does,
    /// fails this on `mint`'s resource id — `custody.ingot` holds nothing, so
    /// no other line repeats it.
    #[test]
    fn table_view_prints_every_id_the_tools_take() {
        let fixture = play_fixture();
        let topology = &fixture.topology;
        let custody = &fixture.custody;
        let fact = fixture.claimed_fact;
        let snapshot = &fixture.snapshot;
        let view = table_view(snapshot);
        let holder_label = snapshot
            .subjects
            .iter()
            .find(|subject| subject.id == custody.holder)
            .expect("the holder is in the snapshot")
            .label
            .clone();
        let mut checked_any = false;
        for name in PLAY_TOOLS {
            let tool = PATCH_TOOLS
                .iter()
                .find(|entry| entry.name == *name)
                .unwrap_or_else(|| panic!("PLAY_TOOLS names `{name}`, missing from PATCH_TOOLS"));
            let mut object = serde_json::Map::new();
            for field in tool.fields {
                let value = match field.kind {
                    patch::PatchFieldKind::Reference(referent)
                    | patch::PatchFieldKind::OptionalReference(referent) => {
                        let id = fixture_id(referent, topology, custody, fact, snapshot);
                        checked_any = true;
                        assert!(
                            view.contains(&id),
                            "table_view is missing {name}'s {referent} id {id}"
                        );
                        existing_ref_value(&id)
                    }
                    patch::PatchFieldKind::ReferenceSet(referent) => {
                        let id = fixture_id(referent, topology, custody, fact, snapshot);
                        checked_any = true;
                        assert!(
                            view.contains(&id),
                            "table_view is missing {name}'s {referent} id {id}"
                        );
                        Value::Array(vec![existing_ref_value(&id)])
                    }
                    patch::PatchFieldKind::Composite(patch::CompositeShape::CommitmentKey) => {
                        let command = id_text(fixture.commitment_key.command);
                        checked_any = true;
                        assert!(
                            view.contains(&command),
                            "table_view is missing {name}'s commitment command id {command}"
                        );
                        // PA.f75 (mutation M8): `contains(&index.to_string())`
                        // passed trivially at index 0, since "0" occurs in
                        // nearly every UUID in the view. Check the exact
                        // `command/index` pairing the Commitments row prints.
                        let key_text = format!("{command}/{}", fixture.commitment_key.index);
                        assert!(
                            view.contains(&key_text),
                            "table_view is missing {name}'s exact commitment key `{key_text}`: {view}"
                        );
                        serde_json::json!({
                            "command": command,
                            "index": fixture.commitment_key.index,
                        })
                    }
                    patch::PatchFieldKind::Composite(patch::CompositeShape::PressureSourceRef) => {
                        let id = id_text(custody.counterparty);
                        checked_any = true;
                        // PA.f75 (mutation M5): the id also appears on the
                        // Subjects line, so a bare `view.contains` passes
                        // even if the Pressures section itself dropped the
                        // source entirely. Check the Pressures section text.
                        assert!(
                            view_section(&view, "\n  Pressures:").contains(&id),
                            "table_view's Pressures section is missing {name}'s pressure source id {id}: {view}"
                        );
                        serde_json::json!({"from": "subject", "of": existing_ref_value(&id)})
                    }
                    patch::PatchFieldKind::Composite(patch::CompositeShape::DependencyRef) => {
                        let id = id_text(custody.counterparty);
                        checked_any = true;
                        // PA.f75 (mutation M6): same reasoning as the
                        // pressure check above — check the holder's own
                        // "depends on" field, not the whole view.
                        let holder_row = subject_row(&view, &holder_label, &id_text(custody.holder));
                        assert!(
                            holder_row.contains(&id),
                            "table_view's holder row is missing {name}'s dependency target id {id}: {holder_row}"
                        );
                        serde_json::json!({"target": "subject", "ref": existing_ref_value(&id)})
                    }
                    other => patch::field_example(other),
                };
                object.insert(field.name.to_owned(), value);
            }
            let arguments = Value::Object(object).to_string();
            decode_authoring_call(name, &arguments)
                .unwrap_or_else(|error| panic!("{name} did not decode its own fixture example: {error}"));
        }
        assert!(checked_any, "no PLAY_TOOLS field referenced a fixture id");

        // The ruled fact prints its real standing, not the `Canonical`
        // collapse every subject-facing knowledge row applies (PA.f47).
        assert!(
            view.contains(&format!("[{}]: ruled;", id_text(fixture.ruled_fact))),
            "table_view does not print the ruled fact's real standing: {view}"
        );
        assert!(
            view.contains(&id_text(fixture.claimed_fact)),
            "table_view is missing the claimed fact's id"
        );

        // PA.f75: `communicate` — the only `PLAY_TOOLS` entry that ever took
        // an `AudienceRef` channel id — was removed, which silently dropped
        // this suite's only coverage of channel printing. Pin it directly:
        // the fixture's declared channel must appear in the Channels
        // section itself, not merely somewhere in the whole view.
        assert!(
            view_section(&view, "\n  Channels:").contains(&id_text(fixture.channel)),
            "table_view's Channels section is missing the declared channel's id: {view}"
        );
    }

    /// The text of one named section of a `table_view` rendering — from just
    /// after `header` up to (not including) the next `"\n  "`-indented
    /// section header. Lets a test assert an id appears in the *right*
    /// section rather than anywhere in the whole view (PA.f75).
    fn view_section<'a>(view: &'a str, header: &str) -> &'a str {
        let start = view
            .find(header)
            .unwrap_or_else(|| panic!("`{header}` section is missing from the view: {view}"));
        let after = &view[start + header.len()..];
        let end = after.find("\n  ").unwrap_or(after.len());
        &after[..end]
    }

    /// The text of one subject's own row — from `"{label} [{id}]"` up to and
    /// including the closing `");"` — so a test can assert a field belongs
    /// to that subject specifically (PA.f75), the same slice
    /// `table_view_prints_only_a_subjects_own_granted_affordances` already
    /// took inline.
    fn subject_row<'a>(view: &'a str, label: &str, id: &str) -> &'a str {
        let marker = format!("{label} [{id}]");
        // Search only from the Subjects section onward: since PA.f74 put an
        // id beside every occupant and knower label too, the same marker can
        // appear earlier, in a place's occupant list or a fact's known-by
        // list, and matching the first occurrence anywhere would return a
        // slice spanning unrelated sections.
        let subjects_start = view.find("\n  Subjects:").unwrap_or(0);
        let search_area = &view[subjects_start..];
        let start = search_area
            .find(&marker)
            .unwrap_or_else(|| panic!("`{marker}` row is missing from the Subjects section: {view}"));
        let rest = &search_area[start..];
        let end = rest.find(");").map_or(rest.len(), |index| index + 2);
        &rest[..end]
    }

    /// Mutation X3: a `Claimed` fact printed as `canonical` instead of
    /// `claimed by <holder>`.
    #[test]
    fn table_view_prints_a_claimed_fact_as_claimed_not_canonical() {
        let fixture = play_fixture();
        let view = table_view(&fixture.snapshot);
        let holder_label = fixture
            .snapshot
            .subjects
            .iter()
            .find(|subject| subject.id == fixture.custody.holder)
            .expect("the holder is in the snapshot")
            .label
            .clone();
        assert!(
            view.contains(&format!(
                "[{}]: claimed by {holder_label};",
                id_text(fixture.claimed_fact)
            )),
            "table_view does not print the claimed fact as claimed by its holder: {view}"
        );
    }

    /// Mutation X4: the "known by" list dropped from a fact's row.
    /// PA.f74: the known-by entry carries the knower's id beside its label,
    /// not the label alone — ambiguous for two subjects sharing one label.
    #[test]
    fn table_view_prints_who_knows_a_fact() {
        let fixture = play_fixture();
        let view = table_view(&fixture.snapshot);
        let holder = fixture
            .snapshot
            .subjects
            .iter()
            .find(|subject| subject.id == fixture.custody.holder)
            .expect("the holder is in the snapshot");
        assert!(
            view.contains(&format!(
                "[{}]: claimed by {}; known by: {} [{}];",
                id_text(fixture.claimed_fact),
                holder.label,
                holder.label,
                id_text(holder.id)
            )),
            "table_view dropped the claimed fact's known-by list, or dropped the knower's id: {view}"
        );
    }

    /// Mutation X1: the granted-affordance filter dropped, so a subject's
    /// `granted:` list prints every affordance in the world rather than only
    /// the ones it actually holds.
    #[test]
    fn table_view_prints_only_a_subjects_own_granted_affordances() {
        let fixture = play_fixture();
        let view = table_view(&fixture.snapshot);
        let holder = fixture
            .snapshot
            .subjects
            .iter()
            .find(|subject| subject.id == fixture.custody.holder)
            .expect("the holder is in the snapshot");
        let ungranted = fixture
            .snapshot
            .affordances
            .iter()
            .find(|affordance| !holder.affordances.contains(&affordance.id))
            .expect("the fixture world declares an affordance the holder was not granted");
        let row = subject_row(&view, &holder.label, &id_text(holder.id));
        assert!(
            !row.contains(&ungranted.entry.kind.0),
            "table_view granted the holder an affordance it does not hold: {row}"
        );
    }

    /// Rule (PA.f74): `table_view` prints a subject's own authority grants,
    /// the offices it holds and grants (with incumbent), and the forums
    /// whose standing covers it (with grievance kind) — what
    /// `create_commitment`'s `authorized`/`has_standing` checks and a
    /// restricted route's `requires` all resolve against. Mutation: drop the
    /// authority section entirely.
    #[test]
    fn table_view_prints_authority_grants_offices_and_forums() {
        let directory = tempfile::tempdir().expect("a temp dir");
        let mut kernel = crate::WorldKernel::create(
            directory.path().join("world.cc"),
            creation(super::CommandId::new(), "PlayCivic"),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let (_, civic, active) = crate::tests::civic_world(&mut kernel);
        let view = table_view(&active);

        let treasury = active
            .subjects
            .iter()
            .find(|subject| subject.id == civic.treasury)
            .expect("the treasury is in the snapshot");
        assert!(!treasury.components.authority.is_empty(), "the treasury holds authority grants");
        for grant in &treasury.components.authority {
            assert!(
                view.contains(&format!("`{}`", grant.kind.0)),
                "table_view is missing the treasury's authority kind `{}`: {view}",
                grant.kind.0
            );
        }
        assert!(!treasury.offices_granted.is_empty(), "the treasury constitutes offices");
        for office in &treasury.offices_granted {
            assert!(
                view.contains(&format!("`{}`", office.office.0)),
                "table_view is missing an office the treasury grants, `{}`: {view}",
                office.office.0
            );
        }

        let reeve = active
            .subjects
            .iter()
            .find(|subject| subject.id == civic.reeve)
            .expect("the reeve is in the snapshot");
        assert!(!reeve.offices_held.is_empty(), "the reeve holds an office");
        for office in &reeve.offices_held {
            assert!(
                view.contains(&format!("`{}`", office.office.0)),
                "table_view is missing an office the reeve holds, `{}`: {view}",
                office.office.0
            );
        }

        let with_redress = active
            .subjects
            .iter()
            .find(|subject| !subject.redress.is_empty())
            .expect("at least one subject holds standing to a forum");
        for forum in &with_redress.redress {
            assert!(
                view.contains(&format!("`{}`", forum.grievance.0)),
                "table_view is missing a forum's grievance kind `{}`: {view}",
                forum.grievance.0
            );
        }
    }

    /// Rule (PA.f74): a commitment's kind prints in plain words, not Rust
    /// `Debug` (`Obligation`).
    #[test]
    fn table_view_prints_the_commitment_kind_in_plain_words() {
        let fixture = play_fixture();
        let view = table_view(&fixture.snapshot);
        assert!(view.contains("obligation"), "{view}");
        assert!(!view.contains("Obligation"), "{view}");
    }

    /// Rule (PA.f74): persona values and memories print in the order they
    /// were authored, not re-sorted the way every other list here is.
    #[test]
    fn table_view_keeps_persona_values_and_memories_in_stored_order() {
        let directory = tempfile::tempdir().expect("a temp dir");
        let mut kernel = crate::WorldKernel::create(
            directory.path().join("world.cc"),
            creation(super::CommandId::new(), "PlayPersonaOrder"),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let topology = admit_topology(&mut kernel);
        let custody = admit_custody(&mut kernel, &topology);
        let before = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &before,
            crate::CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: Vec::new(),
                    operations: vec![ComponentOp::SetPersonaMaterial {
                        subject: patch::Ref::Existing(custody.holder),
                        values: vec![
                            Statement::new("zebra caution").unwrap(),
                            Statement::new("apple pride").unwrap(),
                        ],
                        voice: Statement::new("a level, unhurried voice").unwrap(),
                        memories: vec![
                            Statement::new("the second memory, nine winters back").unwrap(),
                            Statement::new("the first memory, before that").unwrap(),
                        ],
                        reads: Vec::new(),
                    }],
                    evidence: Vec::new(),
                },
            },
        );
        let snapshot = kernel.snapshot().unwrap();
        let view = table_view(&snapshot);
        let zebra = view.find("zebra caution").expect("zebra caution is in the view");
        let apple = view.find("apple pride").expect("apple pride is in the view");
        assert!(
            zebra < apple,
            "table_view sorted persona values instead of keeping stored order: {view}"
        );
        let second = view.find("the second memory").expect("the second memory is in the view");
        let first = view.find("the first memory").expect("the first memory is in the view");
        assert!(
            second < first,
            "table_view sorted persona memories instead of keeping stored order: {view}"
        );
    }

    /// Pin: nothing in `PersonaLane` or the Projector may call `table_view` —
    /// it is the play agent's own omniscient view and must never reach a
    /// Persona prompt.
    #[test]
    fn table_view_never_reaches_persona_or_projector_code() {
        // Every library source file except this one: the structural guard is
        // that no other module — not `controllers.rs` alone, which is only
        // where `PersonaLane` and the Projector happen to live today — ever
        // calls `table_view(`. `PersonaLane`'s own signature takes no text
        // that could carry it in, so a call anywhere outside `table.rs`
        // would have to be a fresh, wrong wire-up.
        const SOURCES: &[(&str, &str)] = &[
            ("action.rs", include_str!("action.rs")),
            ("clock.rs", include_str!("clock.rs")),
            ("consumer.rs", include_str!("consumer.rs")),
            ("controllers.rs", include_str!("controllers.rs")),
            ("cover.rs", include_str!("cover.rs")),
            ("elaboration.rs", include_str!("elaboration.rs")),
            ("journal.rs", include_str!("journal.rs")),
            ("lens.rs", include_str!("lens.rs")),
            ("lib.rs", include_str!("lib.rs")),
            ("local_inference.rs", include_str!("local_inference.rs")),
            ("mailbox.rs", include_str!("mailbox.rs")),
            ("patch.rs", include_str!("patch.rs")),
            ("sdk_inference.rs", include_str!("sdk_inference.rs")),
            ("tool_schema.rs", include_str!("tool_schema.rs")),
            ("vault.rs", include_str!("vault.rs")),
        ];
        for (name, source) in SOURCES {
            assert!(
                !source.contains("table_view("),
                "{name} must not call table::table_view"
            );
        }
    }
}
