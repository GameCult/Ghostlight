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
    self, ComponentOp, Declaration, FactStandingRef, PATCH_TOOLS, PatchToolShape,
};
use super::{
    ActionMismatch, AffordanceSnapshot, CommandId, DecisionInvocation, DecisionOpportunity,
    EntityId, KernelError, Mismatch, Statement, SubjectId, WorldPatch, WorldSnapshot,
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
/// one declaration or one operation, plus any evidence it cites. This is
/// `apply_tool_call`'s own decode arm, factored out so it has one owner;
/// `apply_tool_call` calls it for every `Declare` or `Operate` shape rather
/// than carrying a second copy of the same match.
pub fn decode_authoring_call(name: &str, arguments: &str) -> Result<WorldPatch, String> {
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

/// The tool catalog for one subject's own granted affordances: `catalog_tools`
/// over the entries its current opportunity actually grants. An unknown
/// subject or one with no live opportunity gets an empty catalog rather than
/// an error — there is nothing for it to call.
pub fn actor_tools(prefix: &str, snapshot: &WorldSnapshot, subject: SubjectId) -> Vec<CodexToolDefinition> {
    match subject_opportunity(snapshot, subject) {
        Some((_, granted)) => controllers::catalog_tools(prefix, &granted),
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
pub fn decode_actor_call(
    snapshot: &WorldSnapshot,
    subject: SubjectId,
    kind: &str,
    arguments: &str,
) -> Result<(DecisionOpportunity, DecisionInvocation), String> {
    let (opportunity, granted) = subject_opportunity(snapshot, subject)
        .ok_or_else(|| "this subject holds no exact opportunity".to_owned())?;
    let entry = granted
        .iter()
        .find(|entry| entry.entry.kind.0 == kind)
        .ok_or_else(|| format!("`{kind}` is not granted by this subject's opportunity"))?;
    let mut invocation = controllers::decode_catalog_call(entry, arguments)?;
    if let Ok(Value::Object(fields)) = serde_json::from_str::<Value>(arguments)
        && let Some(display) = fields.get("display").and_then(Value::as_str)
    {
        invocation.display =
            Some(Statement::new(display).ok_or_else(|| "`display` is not canonical text".to_owned())?);
    }
    Ok((opportunity.clone(), invocation))
}

/// A commit refusal in words the agent can act on. `ActionRejected` names the
/// entry precondition, slot, or role that failed; `PatchRejected` names the
/// tool call — by the handle it declared, or by its position among the
/// operation calls — that produced the offending item. Every other
/// `KernelError` has nothing more useful to say than its own `Display`.
pub fn describe_refusal(snapshot: &WorldSnapshot, error: &KernelError) -> String {
    match error {
        KernelError::ActionRejected(mismatches) => mismatches
            .iter()
            .map(describe_action_mismatch)
            .collect::<Vec<_>>()
            .join("; "),
        KernelError::PatchRejected(mismatches) => mismatches
            .iter()
            .map(|mismatch| describe_patch_mismatch(snapshot, mismatch))
            .collect::<Vec<_>>()
            .join("; "),
        other => other.to_string(),
    }
}

fn describe_action_mismatch(mismatch: &ActionMismatch) -> String {
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
            "role `{}` expected a {expected:?} but was bound to a {actual:?}",
            role.0
        ),
        ActionMismatch::ActorNotPresent { precondition } => format!(
            "precondition #{precondition} of the entry failed: the actor was not present"
        ),
        ActionMismatch::TargetUnreachable { precondition } => format!(
            "precondition #{precondition} of the entry failed: the target is unreachable"
        ),
        ActionMismatch::InsufficientHolding { precondition } => format!(
            "precondition #{precondition} of the entry failed: the actor does not hold enough"
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
            "precondition #{precondition} of the entry failed: the actor is not authorized over that target"
        ),
        ActionMismatch::NoStanding { precondition } => format!(
            "precondition #{precondition} of the entry failed: no forum's standing reaches the actor"
        ),
        ActionMismatch::ActorRoleBound => {
            "the invocation bound the reserved `actor` role".to_owned()
        }
        ActionMismatch::DelegationNotMonotone { slot } => format!(
            "effect slot #{slot} would grant authority the actor does not itself hold"
        ),
        ActionMismatch::FactUnknown { precondition } => format!(
            "precondition #{precondition} of the entry failed: the actor does not hold that fact"
        ),
        ActionMismatch::NoAudience { precondition } => format!(
            "precondition #{precondition} of the entry failed: the actor has no audience"
        ),
        ActionMismatch::CannotReach { precondition } => format!(
            "precondition #{precondition} of the entry failed: the addressed subject is not in that audience"
        ),
        ActionMismatch::NotCommitted { precondition } => format!(
            "precondition #{precondition} of the entry failed: the actor holds no such commitment"
        ),
    }
}

/// A `Mismatch`'s own site, read generically off its serialized shape rather
/// than through a fifty-arm match: every variant tags its site under one of
/// `site`, `operation`, `position`, `handle`, or `referent`, and this reads
/// whichever is present. `site`/`operation`/`position` name the offending
/// tool call directly, by the handle it declared or by its position among
/// the operation calls it authored — the one-item-per-call rule
/// `decode_authoring_call` keeps makes that position the call's own index.
fn describe_patch_mismatch(snapshot: &WorldSnapshot, mismatch: &Mismatch) -> String {
    let value = serde_json::to_value(mismatch).unwrap_or(Value::Null);
    let kind = value
        .get("mismatch")
        .and_then(Value::as_str)
        .unwrap_or("mismatch")
        .to_owned();
    let Some(object) = value.as_object() else {
        return kind;
    };
    let site = if let Some(site) = object.get("site") {
        Some(describe_site(site))
    } else if let Some(operation) = object.get("operation").and_then(Value::as_u64) {
        Some(format!("operation tool call #{operation}"))
    } else if let Some(position) = object.get("position").and_then(Value::as_u64) {
        Some(format!("declaration tool call #{position}"))
    } else if let Some(handle) = object.get("handle").and_then(Value::as_str) {
        Some(format!("the call that declared `{handle}`"))
    } else if let Some(referent) = object.get("referent") {
        Some(describe_referent(snapshot, referent))
    } else {
        None
    };
    match site {
        Some(site) => format!("{site}: {kind}"),
        None => kind,
    }
}

fn describe_site(site: &Value) -> String {
    match site.get("site").and_then(Value::as_str) {
        Some("declaration") => {
            let handle = site.get("at").and_then(Value::as_str).unwrap_or("?");
            format!("the call that declared `{handle}`")
        }
        Some("operation") => {
            let index = site.get("at").and_then(Value::as_u64).unwrap_or_default();
            format!("operation tool call #{index}")
        }
        _ => "an unresolved site".to_owned(),
    }
}

/// A `referent` is either a bare draft handle (a plain string, quoted as the
/// call wrote it) or a `RefName` (a tagged `{namespace, ref}`, resolved to
/// the standing thing's label when the reference is canonical). This is the
/// one place a `PatchRejected` mismatch's referent is turned into the same
/// labels `table_view` prints, rather than a second lookup.
fn describe_referent(snapshot: &WorldSnapshot, referent: &Value) -> String {
    if let Some(handle) = referent.as_str() {
        return format!("the reference to `{handle}`");
    }
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
pub(super) fn id_text(id: impl std::fmt::Debug) -> String {
    let text = format!("{id:?}");
    match (text.find('('), text.rfind(')')) {
        (Some(open), Some(close)) if open < close => text[open + 1..close].to_owned(),
        _ => text,
    }
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
/// retired flag, granted entry names, and holdings; the places' occupants;
/// resource ids; every fact with its id, its standing, and who knows it,
/// because `mint`, `transfer`, `witness`, and `acquire_knowledge` all take
/// those ids — but is its own renderer, not a wrapper over it, so the seed
/// prompt's bytes never move when this one changes.
///
/// `FactStanding::Ruled` collapses to `FactStandingView::Canonical` at
/// snapshot construction (`lib.rs`, `snapshot`'s knowledge projection): a
/// subject's own perception cannot tell a ruled fact from an ordinarily
/// canonical one, and every knowledge row in `WorldSnapshot` is built through
/// that same projection, including the rows this reads. `table_view` prints
/// `ruled` facts as `canonical` until that projection carries the real
/// standing through to an omniscient reader; widening it is a `lib.rs`
/// change outside this cut's table.rs-and-two-moved-functions scope.
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
    let joined = |mut items: Vec<String>| {
        if items.is_empty() {
            "none".to_owned()
        } else {
            items.sort();
            items.join(", ")
        }
    };

    let mut out = String::from(
        "Whole-world table (the play agent's own view; reference anything by the id in brackets):\n",
    );

    out.push_str("  Places:");
    if snapshot.places.is_empty() {
        out.push_str(" none");
    }
    for place in &snapshot.places {
        let occupants = joined(
            snapshot
                .subjects
                .iter()
                .filter(|subject| subject.position == Some(place.id))
                .map(|subject| subject.label.clone())
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
        out.push_str(&format!(
            " {} [{}] ({:?}, {}, in {}, retired: {}, granted: {granted}, holdings: {holdings});",
            subject.label,
            id_text(subject.id),
            subject.kind,
            subject
                .controller_mode
                .map_or_else(|| "external".to_owned(), |mode| format!("{mode:?}")),
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

    out.push_str("\n  Facts:");
    let mut facts: std::collections::BTreeMap<EntityId, (String, String, Vec<String>)> =
        std::collections::BTreeMap::new();
    for subject in &snapshot.subjects {
        for row in &subject.knowledge {
            let entry = facts.entry(row.fact).or_insert_with(|| {
                let standing = match &row.standing {
                    super::FactStandingView::Canonical => "canonical".to_owned(),
                    super::FactStandingView::Claimed { by } => {
                        let label = snapshot
                            .subjects
                            .iter()
                            .find(|holder| holder.id == *by)
                            .map_or_else(|| "an unknown subject".to_owned(), |holder| holder.label.clone());
                        format!("claimed by {label}")
                    }
                };
                (row.statement.as_str().to_owned(), standing, Vec::new())
            });
            entry.2.push(subject.label.clone());
        }
    }
    if facts.is_empty() {
        out.push_str(" none");
    }
    for (fact_id, (statement, standing, knowers)) in &facts {
        out.push_str(&format!(
            " \"{statement}\" [{}]: {standing}; known by: {};",
            id_text(*fact_id),
            joined(knowers.clone())
        ));
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
    const PLAY_TOOLS: &[&str] = &[
        "relocate",
        "transfer",
        "consume",
        "mint",
        "bind",
        "release",
        "communicate",
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

    /// A small standing world: the shared topology and custody fixtures (three
    /// places, four routes, four subjects, two resources, one holding), plus
    /// one fact one subject was given knowledge of. `custody.ingot` is left
    /// unheld on purpose, so a resource id in the view can only have come from
    /// the Resources line itself, never a holding.
    fn play_fixture() -> (Topology, Custody, EntityId, WorldSnapshot) {
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
                    declarations: vec![Declaration::Fact(patch::FactDeclaration {
                        handle: patch::DraftHandle::new("ruling"),
                        label: "The Ruling".into(),
                        statement: Statement::new("The gate stands open by decree.").unwrap(),
                        standing: FactStandingRef::Claimed {
                            by: patch::Ref::Existing(custody.holder),
                        },
                    })],
                    operations: vec![ComponentOp::AcquireKnowledge {
                        subject: patch::Ref::Existing(custody.holder),
                        fact: patch::Ref::Draft(patch::DraftHandle::new("ruling")),
                        source: patch::AuthoredSource::Witnessed,
                        confidence: patch::Confidence::Certain,
                    }],
                    evidence: Vec::new(),
                },
            },
        );
        let active = activate(&mut kernel);
        let fact = active
            .subjects
            .iter()
            .find(|subject| subject.id == custody.holder)
            .expect("the holder is in the snapshot")
            .knowledge
            .first()
            .expect("the holder was granted the ruling")
            .fact;
        (topology, custody, fact, active)
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

    /// Mutation M7.2: if `authoring_tools` stopped refusing an unknown name,
    /// this assertion is the one that would no longer hold.
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

    /// Rule: the elaborator's own decode and the table's decode agree, because
    /// the elaborator's arm now calls the table's function rather than
    /// carrying a second copy.
    #[test]
    fn the_elaborator_and_the_table_share_one_decoder() {
        let arguments = serde_json::json!({
            "route": {"ref": "draft", "value": "route"}
        })
        .to_string();
        let mut draft = WorldPatch::default();
        let mut gaps = Vec::new();
        let mut submitted = false;
        crate::elaboration::apply_tool_call(
            "open_route",
            &arguments,
            &mut draft,
            &mut gaps,
            &mut submitted,
        );
        let table_patch = decode_authoring_call("open_route", &arguments)
            .expect("open_route decodes through the table too");
        assert!(gaps.is_empty());
        assert_eq!(draft.operations, table_patch.operations);
        assert!(draft.declarations.is_empty() && table_patch.declarations.is_empty());
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
            decode_actor_call(&active, speech.speaker, "whisper", &arguments)
                .expect("whisper decodes against the speaker's own opportunity");
        assert_eq!(opportunity, expected);
        assert_eq!(
            invocation.speech.as_ref().map(Statement::as_str),
            Some("a whispered warning")
        );
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
        let (opportunity, _) = decode_actor_call(&active, speech.listener, "whisper", &arguments)
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
        let (.., snapshot) = play_fixture();
        let error = KernelError::ActionRejected(vec![ActionMismatch::ActorNotPresent {
            precondition: 2,
        }]);
        let text = describe_refusal(&snapshot, &error);
        assert!(text.contains("precondition #2"), "{text}");
        assert!(text.contains("not present"), "{text}");
    }

    /// Every other `KernelError` renders by its own `Display`.
    #[test]
    fn describe_refusal_falls_back_to_display_for_every_other_error() {
        let (.., snapshot) = play_fixture();
        let error = KernelError::Unauthorized;
        assert_eq!(describe_refusal(&snapshot, &error), error.to_string());
    }

    /// Rule: `table_view` prints every id a `PLAY_TOOLS` example takes.
    /// Mutation M7.3: printing resources by label alone, as the moved
    /// `render_world_structure` still does, fails this on `mint`'s resource
    /// id — `custody.ingot` holds nothing, so no other line repeats it.
    #[test]
    fn table_view_prints_every_id_the_tools_take() {
        let (topology, custody, fact, snapshot) = play_fixture();
        let view = table_view(&snapshot);
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
                        let id = fixture_id(referent, &topology, &custody, fact, &snapshot);
                        checked_any = true;
                        assert!(
                            view.contains(&id),
                            "table_view is missing {name}'s {referent} id {id}"
                        );
                        existing_ref_value(&id)
                    }
                    patch::PatchFieldKind::ReferenceSet(referent) => {
                        let id = fixture_id(referent, &topology, &custody, fact, &snapshot);
                        checked_any = true;
                        assert!(
                            view.contains(&id),
                            "table_view is missing {name}'s {referent} id {id}"
                        );
                        Value::Array(vec![existing_ref_value(&id)])
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
    }

    /// Pin: nothing in `PersonaLane` or the Projector may call `table_view` —
    /// it is the play agent's own omniscient view and must never reach a
    /// Persona prompt.
    #[test]
    fn table_view_never_reaches_persona_or_projector_code() {
        let source = include_str!("controllers.rs");
        assert!(
            !source.contains("table_view"),
            "controllers.rs must not call table::table_view"
        );
    }
}
