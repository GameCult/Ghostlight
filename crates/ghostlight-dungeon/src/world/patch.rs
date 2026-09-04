//! Closed-patch resolution: the one owner of draft handles, reference kinds, and
//! canonical ID allocation for every admission lane.
//!
//! A patch names structure two ways and no other way: an exact canonical ID that
//! already keys a partition, or a draft handle declared in the same patch.
//! Resolution accumulates the complete mismatch set first and allocates only
//! after that set is empty, so a rejected patch never mints an ID.

use super::{
    AffordanceGrant, AffordanceId, AffordanceKind, CommandId, ControllerAssignment, ControllerId,
    DecisionScope, EdgeId, EntityId, NewController, SubjectId, SubjectKind, SubjectState, WorldId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EntityKind {
    Place,
    Resource,
    Fact,
    Channel,
}

/// Exactly the edge kinds that have a record shape.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EdgeKind {
    Route,
}

/// Traversal rule. `Restricted` names a route whose traversal requires authority
/// over its destination; the `Authority` component does not exist yet, so a
/// `Restricted` route currently admits no one.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AccessKind {
    Public,
    Restricted,
}

/// Whole minutes: the kernel's only time unit, so route cost adds and compares
/// with no converter. The valid range is checked, not typed, so a bad cost joins
/// the complete mismatch set instead of failing deserialization.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct Cost(pub(crate) u32);

/// One year of minutes; bounds path sums.
pub(super) const MAX_ROUTE_COST: u32 = 525_600;

/// Where a subject stands. One place, derived from nothing else.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub(crate) struct Position {
    pub(crate) place: EntityId,
}

/// Namespace plus declared kind of a draft handle. A reference states the kind it
/// expects; resolution refuses a handle that answers with another one. A subject
/// expectation carries `None` where the referring position constrains the
/// namespace but not the kind.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "namespace", rename_all = "snake_case")]
pub(crate) enum RefKind {
    Subject(Option<SubjectKind>),
    Entity(EntityKind),
    Edge(EdgeKind),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct DraftHandle(String);

impl DraftHandle {
    pub(super) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

/// A reference is an exact canonical ID or a handle resolved in the same patch.
/// There is no third form and no `From<String>`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "ref", content = "value", rename_all = "snake_case")]
pub(crate) enum Ref<Id> {
    Existing(Id),
    Draft(DraftHandle),
}

/// A referent in any of the three namespaces.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RefName {
    Subject(Ref<SubjectId>),
    Entity(Ref<EntityId>),
    Edge(Ref<EdgeId>),
}

/// Where a failed check appeared. One way to say it, for declarations and
/// operations alike.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Site {
    Declaration(DraftHandle),
    Operation(usize),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct EvidenceRef(String);

/// `position` is the subject's presence: one place it stands in. A subject
/// declared without one is unplaced until a later pass gives placement an owner.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubjectDeclaration {
    pub(crate) handle: DraftHandle,
    pub(crate) label: String,
    pub(crate) kind: SubjectKind,
    pub(crate) controller: NewController,
    pub(crate) affordances: BTreeSet<AffordanceKind>,
    pub(crate) position: Option<Ref<EntityId>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct EntityDeclaration {
    pub(crate) handle: DraftHandle,
    pub(crate) label: String,
    pub(crate) kind: EntityKind,
    /// Legal only for `EntityKind::Place`, and only ever names a place.
    pub(crate) container: Option<Ref<EntityId>>,
}

/// A declared route is open. A route that should start closed is `declare` plus
/// `CloseRoute` in the same patch, so `open` has one writer family.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct RouteDeclaration {
    pub(crate) handle: DraftHandle,
    pub(crate) label: String,
    pub(crate) from: Ref<EntityId>,
    pub(crate) to: Ref<EntityId>,
    pub(crate) access: AccessKind,
    pub(crate) cost: Cost,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Declaration {
    Subject(SubjectDeclaration),
    Entity(EntityDeclaration),
    Route(RouteDeclaration),
}

/// The operations that change a component of an already canonical structure.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub(crate) enum ComponentOp {
    Relocate {
        subject: Ref<SubjectId>,
        via: Ref<EdgeId>,
    },
    OpenRoute {
        route: Ref<EdgeId>,
    },
    CloseRoute {
        route: Ref<EdgeId>,
    },
    AlterCost {
        route: Ref<EdgeId>,
        cost: Cost,
    },
}

/// Empty on purpose: an `Option<PatchAnswer>` can only be `None`, so the
/// answers-are-None rule is compile-enforced.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum PatchAnswer {}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorldPatch {
    pub(crate) declarations: Vec<Declaration>,
    pub(crate) operations: Vec<ComponentOp>,
    pub(crate) evidence: Vec<EvidenceRef>,
}

/// One named structural check that a patch failed. A rejection carries the
/// complete set, never the first failure, and is never persisted.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Mismatch {
    EmptyHandle {
        position: usize,
    },
    DuplicateHandle {
        handle: DraftHandle,
    },
    EmptyLabel {
        handle: DraftHandle,
    },
    UnresolvedDraft {
        site: Site,
        referent: DraftHandle,
        expected: RefKind,
    },
    WrongKind {
        site: Site,
        referent: RefName,
        expected: RefKind,
        actual: RefKind,
    },
    UnknownCanonical {
        site: Site,
        expected: RefKind,
    },
    UnadmittedController {
        handle: DraftHandle,
    },
    NoAffordances {
        handle: DraftHandle,
    },
    EmptyEvidence {
        position: usize,
    },
    CanonicalCollision {
        handle: DraftHandle,
    },
    NoCanonicalChange,
    /// A genesis-lane resolution (the same empty-state case `admits_human`
    /// tests) declared no subject, so the world would have no decision
    /// subject to admit. This is a rejected patch, not a corrupt journal: the
    /// mailbox actor must survive it.
    NoDecisionSubject,
    /// A declared place contains itself through its container chain.
    ContainmentCycle {
        referent: DraftHandle,
    },
    RouteSelfLoop {
        referent: DraftHandle,
    },
    InvalidCost {
        site: Site,
    },
    SubjectNotAtOrigin {
        operation: usize,
    },
    RouteClosed {
        operation: usize,
    },
    RouteAccessRestricted {
        operation: usize,
    },
    UnplacedSubject {
        operation: usize,
    },
    /// An operation that changes nothing is not a canonical change.
    NoOperationEffect {
        operation: usize,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct EntityRecord {
    pub(super) label: String,
    pub(super) kind: EntityKind,
    /// `Some` only for `EntityKind::Place`, and only ever a place.
    pub(super) container: Option<EntityId>,
}

/// One variant per edge kind that has a shape, so no record carries fields that
/// are meaningless for its discriminator.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum EdgeRecord {
    Route {
        label: String,
        from: EntityId,
        to: EntityId,
        access: AccessKind,
        cost: Cost,
        open: bool,
    },
}

impl EdgeRecord {
    pub(super) fn endpoints(&self) -> (EntityId, EntityId) {
        match self {
            Self::Route { from, to, .. } => (*from, *to),
        }
    }

    pub(super) fn label(&self) -> &str {
        match self {
            Self::Route { label, .. } => label,
        }
    }

    pub(super) fn cost(&self) -> Cost {
        match self {
            Self::Route { cost, .. } => *cost,
        }
    }

    pub(super) fn access(&self) -> AccessKind {
        match self {
            Self::Route { access, .. } => *access,
        }
    }

    pub(super) fn is_open(&self) -> bool {
        match self {
            Self::Route { open, .. } => *open,
        }
    }

    pub(super) fn set_open(&mut self, value: bool) {
        match self {
            Self::Route { open, .. } => *open = value,
        }
    }

    pub(super) fn set_cost(&mut self, value: Cost) {
        match self {
            Self::Route { cost, .. } => *cost = value,
        }
    }
}

/// The handle-to-ID binding table for one subject. The handle is commit-scoped
/// provenance; it never enters `WorldState`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct ResolvedSubject {
    pub(super) handle: DraftHandle,
    pub(super) subject_id: SubjectId,
    pub(super) subject: SubjectState,
    pub(super) controller: ControllerAssignment,
    pub(super) affordances: BTreeMap<AffordanceId, AffordanceGrant>,
    pub(super) position: Option<Position>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct ResolvedEntity {
    pub(super) handle: DraftHandle,
    pub(super) entity_id: EntityId,
    pub(super) entity: EntityRecord,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct ResolvedRoute {
    pub(super) handle: DraftHandle,
    pub(super) edge_id: EdgeId,
    pub(super) edge: EdgeRecord,
}

/// A lowered operation. `Relocate` carries no endpoints: they are read from the
/// route at apply time, so a forged effect cannot assert a destination the route
/// does not have.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub(super) enum ResolvedOp {
    Relocate {
        subject_id: SubjectId,
        edge_id: EdgeId,
    },
    OpenRoute {
        edge_id: EdgeId,
    },
    CloseRoute {
        edge_id: EdgeId,
    },
    AlterCost {
        edge_id: EdgeId,
        cost: Cost,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct ResolvedPatch {
    pub(super) subjects: Vec<ResolvedSubject>,
    pub(super) entities: Vec<ResolvedEntity>,
    pub(super) routes: Vec<ResolvedRoute>,
    pub(super) operations: Vec<ResolvedOp>,
    pub(super) evidence: Vec<EvidenceRef>,
}

impl ResolvedPatch {
    pub(super) fn declares_nothing(&self) -> bool {
        self.subjects.is_empty()
            && self.entities.is_empty()
            && self.routes.is_empty()
            && self.evidence.is_empty()
    }
}

const SUBJECT_NAMESPACE: &str = "ghostlight.id.subject.v1";
const ENTITY_NAMESPACE: &str = "ghostlight.id.entity.v1";
const EDGE_NAMESPACE: &str = "ghostlight.id.edge.v1";
const CONTROLLER_NAMESPACE: &str = "ghostlight.id.controller.v1";
const AFFORDANCE_NAMESPACE: &str = "ghostlight.id.affordance.v1";

const PLACE: RefKind = RefKind::Entity(EntityKind::Place);
const ROUTE: RefKind = RefKind::Edge(EdgeKind::Route);
const ANY_SUBJECT: RefKind = RefKind::Subject(None);

/// A candidate-graph key: a structure this patch declares, or one the world
/// already holds. Candidate topology is checked before any ID is minted, so it
/// is keyed by reference rather than by canonical ID.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key<Id> {
    Existing(Id),
    Draft(DraftHandle),
}

fn key_of<Id: Copy>(reference: &Ref<Id>) -> Key<Id> {
    match reference {
        Ref::Existing(id) => Key::Existing(*id),
        Ref::Draft(handle) => Key::Draft(handle.clone()),
    }
}

#[derive(Clone, Debug)]
struct RouteCandidate {
    from: Key<EntityId>,
    to: Key<EntityId>,
    access: AccessKind,
    cost: Cost,
    open: bool,
}

/// The only canonical ID allocator. Deterministic on purpose: journal replay
/// recomputes `reduce` and requires effect equality, so a reduce arm can never
/// draw a `Uuid::new_v4`. Preimage fields are length-prefixed where they are
/// variable-width, so the concatenation is unambiguous.
fn derive_id(
    namespace: &str,
    world_id: WorldId,
    command_id: CommandId,
    handle: &DraftHandle,
    discriminator: Option<&str>,
) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(namespace.as_bytes());
    hasher.update(world_id.0.as_bytes());
    hasher.update(command_id.0.as_bytes());
    hasher.update((handle.0.len() as u64).to_be_bytes());
    hasher.update(handle.0.as_bytes());
    if let Some(discriminator) = discriminator {
        hasher.update(discriminator.as_bytes());
    }
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&hasher.finalize()[..16]);
    Uuid::from_bytes(bytes)
}

fn affordance_slug(kind: AffordanceKind) -> &'static str {
    match kind {
        AffordanceKind::Speak => "speak",
    }
}

pub(super) fn is_canonical_text(value: &str) -> bool {
    !value.trim().is_empty() && value.trim() == value
}

pub(super) fn is_valid_cost(cost: Cost) -> bool {
    (1..=MAX_ROUTE_COST).contains(&cost.0)
}

fn resolve_place(
    site: Site,
    reference: &Ref<EntityId>,
    index: &BTreeMap<DraftHandle, RefKind>,
    entities: &BTreeMap<EntityId, EntityRecord>,
    mismatches: &mut Vec<Mismatch>,
) -> Option<Key<EntityId>> {
    match reference {
        Ref::Draft(named) => match index.get(named) {
            None => {
                mismatches.push(Mismatch::UnresolvedDraft {
                    site,
                    referent: named.clone(),
                    expected: PLACE,
                });
                None
            }
            Some(kind) if *kind != PLACE => {
                mismatches.push(Mismatch::WrongKind {
                    site,
                    referent: RefName::Entity(reference.clone()),
                    expected: PLACE,
                    actual: *kind,
                });
                None
            }
            Some(_) => Some(Key::Draft(named.clone())),
        },
        Ref::Existing(entity_id) => match entities.get(entity_id) {
            None => {
                mismatches.push(Mismatch::UnknownCanonical {
                    site,
                    expected: PLACE,
                });
                None
            }
            Some(record) if record.kind != EntityKind::Place => {
                mismatches.push(Mismatch::WrongKind {
                    site,
                    referent: RefName::Entity(reference.clone()),
                    expected: PLACE,
                    actual: RefKind::Entity(record.kind),
                });
                None
            }
            Some(_) => Some(Key::Existing(*entity_id)),
        },
    }
}

fn resolve_subject(
    site: Site,
    reference: &Ref<SubjectId>,
    index: &BTreeMap<DraftHandle, RefKind>,
    subjects: &BTreeMap<SubjectId, SubjectState>,
    mismatches: &mut Vec<Mismatch>,
) -> Option<Key<SubjectId>> {
    match reference {
        Ref::Draft(named) => match index.get(named) {
            None => {
                mismatches.push(Mismatch::UnresolvedDraft {
                    site,
                    referent: named.clone(),
                    expected: ANY_SUBJECT,
                });
                None
            }
            Some(RefKind::Subject(_)) => Some(Key::Draft(named.clone())),
            Some(kind) => {
                mismatches.push(Mismatch::WrongKind {
                    site,
                    referent: RefName::Subject(reference.clone()),
                    expected: ANY_SUBJECT,
                    actual: *kind,
                });
                None
            }
        },
        Ref::Existing(subject_id) => {
            if subjects.contains_key(subject_id) {
                Some(Key::Existing(*subject_id))
            } else {
                mismatches.push(Mismatch::UnknownCanonical {
                    site,
                    expected: ANY_SUBJECT,
                });
                None
            }
        }
    }
}

fn resolve_route(
    site: Site,
    reference: &Ref<EdgeId>,
    index: &BTreeMap<DraftHandle, RefKind>,
    edges: &BTreeMap<EdgeId, EdgeRecord>,
    mismatches: &mut Vec<Mismatch>,
) -> Option<Key<EdgeId>> {
    match reference {
        Ref::Draft(named) => match index.get(named) {
            None => {
                mismatches.push(Mismatch::UnresolvedDraft {
                    site,
                    referent: named.clone(),
                    expected: ROUTE,
                });
                None
            }
            Some(kind) if *kind != ROUTE => {
                mismatches.push(Mismatch::WrongKind {
                    site,
                    referent: RefName::Edge(reference.clone()),
                    expected: ROUTE,
                    actual: *kind,
                });
                None
            }
            Some(_) => Some(Key::Draft(named.clone())),
        },
        Ref::Existing(edge_id) => {
            if edges.contains_key(edge_id) {
                Some(Key::Existing(*edge_id))
            } else {
                mismatches.push(Mismatch::UnknownCanonical {
                    site,
                    expected: ROUTE,
                });
                None
            }
        }
    }
}

/// The one resolution owner for every admission lane: declarations, references,
/// topology admission, and operation preconditions. Every check runs against the
/// complete candidate graph — what the world already holds plus what this patch
/// declares — and the whole mismatch set closes before the first ID is minted.
pub(super) fn resolve_patch(
    state: &super::WorldState,
    command_id: CommandId,
    patch: &WorldPatch,
) -> Result<ResolvedPatch, Vec<Mismatch>> {
    let world_id = state.world_id;
    let admits_human = super::admits_human(state.revision, &state.subjects);
    let mut mismatches = Vec::new();
    let mut index: BTreeMap<DraftHandle, RefKind> = BTreeMap::new();

    for (position, declaration) in patch.declarations.iter().enumerate() {
        let (handle, label, kind) = match declaration {
            Declaration::Subject(subject) => (
                &subject.handle,
                &subject.label,
                RefKind::Subject(Some(subject.kind)),
            ),
            Declaration::Entity(entity) => {
                (&entity.handle, &entity.label, RefKind::Entity(entity.kind))
            }
            Declaration::Route(route) => (&route.handle, &route.label, ROUTE),
        };
        let named = is_canonical_text(&handle.0);
        if !named {
            mismatches.push(Mismatch::EmptyHandle { position });
        }
        if !is_canonical_text(label) {
            mismatches.push(Mismatch::EmptyLabel {
                handle: handle.clone(),
            });
        }
        if named {
            match index.entry(handle.clone()) {
                Entry::Vacant(slot) => {
                    slot.insert(kind);
                }
                Entry::Occupied(_) => mismatches.push(Mismatch::DuplicateHandle {
                    handle: handle.clone(),
                }),
            }
        }
        if let Declaration::Subject(subject) = declaration {
            if subject.affordances.is_empty() {
                mismatches.push(Mismatch::NoAffordances {
                    handle: handle.clone(),
                });
            }
            if let NewController::Human { principal } = &subject.controller
                && (!admits_human || super::validate_principal(principal).is_err())
            {
                mismatches.push(Mismatch::UnadmittedController {
                    handle: handle.clone(),
                });
            }
        }
    }

    for (position, evidence) in patch.evidence.iter().enumerate() {
        if !is_canonical_text(&evidence.0) {
            mismatches.push(Mismatch::EmptyEvidence { position });
        }
    }

    // The candidate graph: canonical topology plus everything this patch
    // declares. Containment, endpoints, and preconditions all read it.
    let mut containers: BTreeMap<Key<EntityId>, Key<EntityId>> = state
        .entities
        .iter()
        .filter_map(|(entity_id, record)| {
            record
                .container
                .map(|container| (Key::Existing(*entity_id), Key::Existing(container)))
        })
        .collect();
    let mut routes: BTreeMap<Key<EdgeId>, RouteCandidate> = state
        .edges
        .iter()
        .map(|(edge_id, record)| {
            let (from, to) = record.endpoints();
            (
                Key::Existing(*edge_id),
                RouteCandidate {
                    from: Key::Existing(from),
                    to: Key::Existing(to),
                    access: record.access(),
                    cost: record.cost(),
                    open: record.is_open(),
                },
            )
        })
        .collect();
    let mut positions: BTreeMap<Key<SubjectId>, Key<EntityId>> = state
        .positions
        .iter()
        .map(|(subject_id, position)| (Key::Existing(*subject_id), Key::Existing(position.place)))
        .collect();
    let mut declared_places: Vec<(DraftHandle, Key<EntityId>)> = Vec::new();

    for declaration in &patch.declarations {
        match declaration {
            Declaration::Entity(entity) => {
                if entity.kind == EntityKind::Place {
                    declared_places
                        .push((entity.handle.clone(), Key::Draft(entity.handle.clone())));
                }
                if let Some(reference) = &entity.container {
                    if entity.kind != EntityKind::Place {
                        mismatches.push(Mismatch::WrongKind {
                            site: Site::Declaration(entity.handle.clone()),
                            referent: RefName::Entity(reference.clone()),
                            expected: PLACE,
                            actual: RefKind::Entity(entity.kind),
                        });
                    } else if let Some(parent) = resolve_place(
                        Site::Declaration(entity.handle.clone()),
                        reference,
                        &index,
                        &state.entities,
                        &mut mismatches,
                    ) {
                        containers.insert(Key::Draft(entity.handle.clone()), parent);
                    }
                }
            }
            Declaration::Subject(subject) => {
                if let Some(reference) = &subject.position
                    && let Some(place) = resolve_place(
                        Site::Declaration(subject.handle.clone()),
                        reference,
                        &index,
                        &state.entities,
                        &mut mismatches,
                    )
                {
                    positions.insert(Key::Draft(subject.handle.clone()), place);
                }
            }
            Declaration::Route(route) => {
                let from = resolve_place(
                    Site::Declaration(route.handle.clone()),
                    &route.from,
                    &index,
                    &state.entities,
                    &mut mismatches,
                );
                let to = resolve_place(
                    Site::Declaration(route.handle.clone()),
                    &route.to,
                    &index,
                    &state.entities,
                    &mut mismatches,
                );
                if !is_valid_cost(route.cost) {
                    mismatches.push(Mismatch::InvalidCost {
                        site: Site::Declaration(route.handle.clone()),
                    });
                }
                if let (Some(from), Some(to)) = (from, to) {
                    if from == to {
                        mismatches.push(Mismatch::RouteSelfLoop {
                            referent: route.handle.clone(),
                        });
                    }
                    routes.insert(
                        Key::Draft(route.handle.clone()),
                        RouteCandidate {
                            from,
                            to,
                            access: route.access,
                            cost: route.cost,
                            open: true,
                        },
                    );
                }
            }
        }
    }

    // Containment acyclicity. Containment is immutable after admission and
    // admission already refused cycles, so every cycle contains a declared
    // place; walking from each declared place names them all.
    for (handle, start) in &declared_places {
        if contains_itself(start, &containers) {
            mismatches.push(Mismatch::ContainmentCycle {
                referent: handle.clone(),
            });
        }
    }

    for (position, operation) in patch.operations.iter().enumerate() {
        match operation {
            ComponentOp::Relocate { subject, via } => {
                let subject_key = resolve_subject(
                    Site::Operation(position),
                    subject,
                    &index,
                    &state.subjects,
                    &mut mismatches,
                );
                let route_key = resolve_route(
                    Site::Operation(position),
                    via,
                    &index,
                    &state.edges,
                    &mut mismatches,
                );
                let (Some(subject_key), Some(route_key)) = (subject_key, route_key) else {
                    continue;
                };
                let Some(route) = routes.get(&route_key).cloned() else {
                    continue;
                };
                let mut admitted = true;
                match positions.get(&subject_key) {
                    None => {
                        mismatches.push(Mismatch::UnplacedSubject {
                            operation: position,
                        });
                        admitted = false;
                    }
                    Some(place) if *place != route.from => {
                        mismatches.push(Mismatch::SubjectNotAtOrigin {
                            operation: position,
                        });
                        admitted = false;
                    }
                    Some(_) => {}
                }
                if !route.open {
                    mismatches.push(Mismatch::RouteClosed {
                        operation: position,
                    });
                    admitted = false;
                }
                if route.access != AccessKind::Public {
                    mismatches.push(Mismatch::RouteAccessRestricted {
                        operation: position,
                    });
                    admitted = false;
                }
                if admitted {
                    positions.insert(subject_key, route.to);
                }
            }
            ComponentOp::OpenRoute { route } | ComponentOp::CloseRoute { route } => {
                let open = matches!(operation, ComponentOp::OpenRoute { .. });
                let Some(route_key) = resolve_route(
                    Site::Operation(position),
                    route,
                    &index,
                    &state.edges,
                    &mut mismatches,
                ) else {
                    continue;
                };
                let Some(candidate) = routes.get_mut(&route_key) else {
                    continue;
                };
                if candidate.open == open {
                    mismatches.push(Mismatch::NoOperationEffect {
                        operation: position,
                    });
                } else {
                    candidate.open = open;
                }
            }
            ComponentOp::AlterCost { route, cost } => {
                let route_key = resolve_route(
                    Site::Operation(position),
                    route,
                    &index,
                    &state.edges,
                    &mut mismatches,
                );
                if !is_valid_cost(*cost) {
                    mismatches.push(Mismatch::InvalidCost {
                        site: Site::Operation(position),
                    });
                    continue;
                }
                let Some(route_key) = route_key else { continue };
                let Some(candidate) = routes.get_mut(&route_key) else {
                    continue;
                };
                if candidate.cost == *cost {
                    mismatches.push(Mismatch::NoOperationEffect {
                        operation: position,
                    });
                } else {
                    candidate.cost = *cost;
                }
            }
        }
    }

    if patch.declarations.is_empty() && patch.operations.is_empty() {
        mismatches.push(Mismatch::NoCanonicalChange);
    }

    if admits_human
        && !patch
            .declarations
            .iter()
            .any(|declaration| matches!(declaration, Declaration::Subject(_)))
    {
        mismatches.push(Mismatch::NoDecisionSubject);
    }

    if !mismatches.is_empty() {
        mismatches.sort();
        return Err(mismatches);
    }

    // No canonical ID exists above this line. Allocation starts only now.
    let mut collisions = Vec::new();
    let mut entities = Vec::new();
    let mut allocated_entities: BTreeMap<DraftHandle, EntityId> = BTreeMap::new();
    for declaration in &patch.declarations {
        let Declaration::Entity(entity) = declaration else {
            continue;
        };
        let entity_id = EntityId(derive_id(
            ENTITY_NAMESPACE,
            world_id,
            command_id,
            &entity.handle,
            None,
        ));
        if state.entities.contains_key(&entity_id) {
            collisions.push(Mismatch::CanonicalCollision {
                handle: entity.handle.clone(),
            });
        }
        allocated_entities.insert(entity.handle.clone(), entity_id);
        entities.push(ResolvedEntity {
            handle: entity.handle.clone(),
            entity_id,
            entity: EntityRecord {
                label: entity.label.clone(),
                kind: entity.kind,
                container: None,
            },
        });
    }
    let place_id = |key: &Key<EntityId>| -> EntityId {
        match key {
            Key::Existing(entity_id) => *entity_id,
            Key::Draft(handle) => *allocated_entities
                .get(handle)
                .expect("a draft place reference resolved above"),
        }
    };
    for (resolved, declaration) in entities
        .iter_mut()
        .zip(
            patch
                .declarations
                .iter()
                .filter_map(|declaration| match declaration {
                    Declaration::Entity(entity) => Some(entity),
                    _ => None,
                }),
        )
    {
        resolved.entity.container = declaration
            .container
            .as_ref()
            .map(|reference| place_id(&key_of(reference)));
    }

    let mut allocated_routes: BTreeMap<DraftHandle, EdgeId> = BTreeMap::new();
    let mut routes = Vec::new();
    for declaration in &patch.declarations {
        let Declaration::Route(route) = declaration else {
            continue;
        };
        let edge_id = EdgeId(derive_id(
            EDGE_NAMESPACE,
            world_id,
            command_id,
            &route.handle,
            None,
        ));
        if state.edges.contains_key(&edge_id) {
            collisions.push(Mismatch::CanonicalCollision {
                handle: route.handle.clone(),
            });
        }
        allocated_routes.insert(route.handle.clone(), edge_id);
        routes.push(ResolvedRoute {
            handle: route.handle.clone(),
            edge_id,
            edge: EdgeRecord::Route {
                label: route.label.clone(),
                from: place_id(&key_of(&route.from)),
                to: place_id(&key_of(&route.to)),
                access: route.access,
                cost: route.cost,
                open: true,
            },
        });
    }

    let mut allocated_subjects: BTreeMap<DraftHandle, SubjectId> = BTreeMap::new();
    let mut subjects = Vec::new();
    for declaration in &patch.declarations {
        let Declaration::Subject(input) = declaration else {
            continue;
        };
        let subject_id = SubjectId(derive_id(
            SUBJECT_NAMESPACE,
            world_id,
            command_id,
            &input.handle,
            None,
        ));
        if state.subjects.contains_key(&subject_id) {
            collisions.push(Mismatch::CanonicalCollision {
                handle: input.handle.clone(),
            });
        }
        allocated_subjects.insert(input.handle.clone(), subject_id);
        let scope = DecisionScope { subject_id };
        let controller_id = ControllerId(derive_id(
            CONTROLLER_NAMESPACE,
            world_id,
            command_id,
            &input.handle,
            None,
        ));
        let controller = match input.controller.clone() {
            NewController::Human { principal } => ControllerAssignment::Human {
                controller_id,
                principal,
            },
            NewController::NarrativePersona => {
                ControllerAssignment::NarrativePersona { controller_id }
            }
            NewController::OperationalAgent => {
                ControllerAssignment::OperationalAgent { controller_id }
            }
        };
        let affordances = input
            .affordances
            .iter()
            .map(|kind| {
                let affordance_id = AffordanceId(derive_id(
                    AFFORDANCE_NAMESPACE,
                    world_id,
                    command_id,
                    &input.handle,
                    Some(affordance_slug(*kind)),
                ));
                (affordance_id, AffordanceGrant { scope, kind: *kind })
            })
            .collect();
        subjects.push(ResolvedSubject {
            handle: input.handle.clone(),
            subject_id,
            subject: SubjectState {
                label: input.label.clone(),
                kind: input.kind,
            },
            controller,
            affordances,
            position: input.position.as_ref().map(|reference| Position {
                place: place_id(&key_of(reference)),
            }),
        });
    }

    let subject_id_of = |key: &Key<SubjectId>| -> SubjectId {
        match key {
            Key::Existing(subject_id) => *subject_id,
            Key::Draft(handle) => *allocated_subjects
                .get(handle)
                .expect("a draft subject reference resolved above"),
        }
    };
    let edge_id_of = |key: &Key<EdgeId>| -> EdgeId {
        match key {
            Key::Existing(edge_id) => *edge_id,
            Key::Draft(handle) => *allocated_routes
                .get(handle)
                .expect("a draft route reference resolved above"),
        }
    };
    let operations = patch
        .operations
        .iter()
        .map(|operation| match operation {
            ComponentOp::Relocate { subject, via } => ResolvedOp::Relocate {
                subject_id: subject_id_of(&key_of(subject)),
                edge_id: edge_id_of(&key_of(via)),
            },
            ComponentOp::OpenRoute { route } => ResolvedOp::OpenRoute {
                edge_id: edge_id_of(&key_of(route)),
            },
            ComponentOp::CloseRoute { route } => ResolvedOp::CloseRoute {
                edge_id: edge_id_of(&key_of(route)),
            },
            ComponentOp::AlterCost { route, cost } => ResolvedOp::AlterCost {
                edge_id: edge_id_of(&key_of(route)),
                cost: *cost,
            },
        })
        .collect();

    if !collisions.is_empty() {
        collisions.sort();
        return Err(collisions);
    }

    Ok(ResolvedPatch {
        subjects,
        entities,
        routes,
        operations,
        evidence: patch.evidence.clone(),
    })
}

/// Whether a place reaches itself through its container chain. The walk is
/// bounded by the graph size, so a cycle that does not include the start still
/// terminates.
fn contains_itself(
    start: &Key<EntityId>,
    containers: &BTreeMap<Key<EntityId>, Key<EntityId>>,
) -> bool {
    let mut current = containers.get(start);
    for _ in 0..=containers.len() {
        match current {
            None => return false,
            Some(node) if node == start => return true,
            Some(node) => current = containers.get(node),
        }
    }
    false
}

/// The same containment walk over canonical state, for the admission and replay
/// invariants. `admit_resolved` runs it after inserting a place, so a container
/// chain that never terminates dies before the partition keeps it.
pub(super) fn containment_terminates(
    start: EntityId,
    entities: &BTreeMap<EntityId, EntityRecord>,
) -> bool {
    let mut current = entities.get(&start).and_then(|record| record.container);
    for _ in 0..=entities.len() {
        match current {
            None => return true,
            Some(node) if node == start => return false,
            Some(node) => current = entities.get(&node).and_then(|record| record.container),
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::tests::{
        activate, admit_topology, auth_principal, command, creation, owner, player, submit_owner,
    };
    use crate::world::{
        CallerId, CommandBody, CommandId, KernelError, SubmitReceipt, WorldKernel, WorldPhase,
    };

    fn entity(handle: &str, label: &str, kind: EntityKind) -> Declaration {
        Declaration::Entity(EntityDeclaration {
            handle: DraftHandle::new(handle),
            label: label.into(),
            kind,
            container: None,
        })
    }

    fn contained(handle: &str, label: &str, container: &str) -> Declaration {
        Declaration::Entity(EntityDeclaration {
            handle: DraftHandle::new(handle),
            label: label.into(),
            kind: EntityKind::Place,
            container: Some(Ref::Draft(DraftHandle::new(container))),
        })
    }

    fn route(handle: &str, label: &str, from: &str, to: &str, cost: u32) -> Declaration {
        Declaration::Route(RouteDeclaration {
            handle: DraftHandle::new(handle),
            label: label.into(),
            from: Ref::Draft(DraftHandle::new(from)),
            to: Ref::Draft(DraftHandle::new(to)),
            access: AccessKind::Public,
            cost: Cost(cost),
        })
    }

    fn institution(handle: &str, label: &str, position: Option<Ref<EntityId>>) -> Declaration {
        Declaration::Subject(SubjectDeclaration {
            handle: DraftHandle::new(handle),
            label: label.into(),
            kind: SubjectKind::Institution,
            controller: NewController::OperationalAgent,
            affordances: BTreeSet::from([AffordanceKind::Speak]),
            position,
        })
    }

    fn patch_of(declarations: Vec<Declaration>) -> WorldPatch {
        WorldPatch {
            declarations,
            operations: Vec::new(),
            evidence: Vec::new(),
        }
    }

    fn operations_of(operations: Vec<ComponentOp>) -> WorldPatch {
        WorldPatch {
            declarations: Vec::new(),
            operations,
            evidence: Vec::new(),
        }
    }

    fn draft(handle: &str) -> DraftHandle {
        DraftHandle::new(handle)
    }

    fn at(handle: &str) -> Site {
        Site::Declaration(DraftHandle::new(handle))
    }

    fn admit(patch: WorldPatch) -> CommandBody {
        CommandBody::AdmitPatch {
            answers: None,
            patch,
        }
    }

    fn draft_world(directory: &std::path::Path) -> WorldKernel {
        WorldKernel::create(
            directory.join("world.cc"),
            creation(CommandId::new(), "Kharad"),
            &auth_principal(owner()),
        )
        .expect("draft world")
        .0
    }

    /// The position reference names a place that no declaration and no partition
    /// provides. Nothing commits and nothing is allocated.
    #[test]
    fn a_subject_declared_at_an_undeclared_place_is_rejected_and_allocates_nothing() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let before = kernel.snapshot().unwrap();
        let commits_before = kernel.journal.commit_count();

        let error = kernel
            .submit(
                command(
                    &before,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    admit(patch_of(vec![institution(
                        "rhythm-authority",
                        "The Rhythm Authority",
                        Some(Ref::Draft(DraftHandle::new("kharad-rhythm-road"))),
                    )])),
                ),
                &auth_principal(owner()),
            )
            .unwrap_err();

        let KernelError::PatchRejected(mismatches) = error else {
            panic!("expected a rejected patch");
        };
        assert_eq!(
            mismatches,
            vec![Mismatch::UnresolvedDraft {
                site: at("rhythm-authority"),
                referent: draft("kharad-rhythm-road"),
                expected: PLACE,
            }]
        );
        assert_eq!(kernel.snapshot().unwrap(), before);
        assert_eq!(kernel.journal.commit_count(), commits_before);
        assert!(kernel.state.entities.is_empty());
        assert_eq!(kernel.state.subjects.len(), before.subjects.len());
    }

    /// A place, a route into it, and a subject standing there are one atomic
    /// commit: every reference resolves to an ID the same patch allocated.
    #[test]
    fn a_declared_place_and_its_dependents_commit_atomically() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let before = kernel.snapshot().unwrap();
        let commits_before = kernel.journal.commit_count();

        let receipt = submit_owner(
            &mut kernel,
            &before,
            admit(patch_of(vec![
                entity("cavity-yard", "The Cavity Yard", EntityKind::Place),
                entity("kharad-rhythm-road", "The Rhythm Road", EntityKind::Place),
                route(
                    "yard-road",
                    "The Yard Ramp",
                    "cavity-yard",
                    "kharad-rhythm-road",
                    12,
                ),
                institution(
                    "rhythm-authority",
                    "The Rhythm Authority",
                    Some(Ref::Draft(draft("kharad-rhythm-road"))),
                ),
            ])),
        );
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));

        let after = kernel.snapshot().unwrap();
        assert_eq!(after.revision, before.revision + 1);
        assert_eq!(kernel.journal.commit_count(), commits_before + 1);
        assert_eq!(kernel.state.entities.len(), 2);
        let road = *kernel
            .state
            .entities
            .iter()
            .find(|(_, record)| record.label == "The Rhythm Road")
            .expect("the road is admitted")
            .0;
        let yard = *kernel
            .state
            .entities
            .iter()
            .find(|(_, record)| record.label == "The Cavity Yard")
            .expect("the yard is admitted")
            .0;
        let (_, edge) = kernel.state.edges.iter().next().expect("the route lands");
        assert_eq!(edge.endpoints(), (yard, road));
        assert_eq!(edge.cost(), Cost(12));
        assert!(edge.is_open());
        let admitted = *kernel
            .state
            .subjects
            .iter()
            .find(|(_, subject)| subject.label == "The Rhythm Authority")
            .expect("the institution is admitted")
            .0;
        assert_eq!(
            kernel.state.positions.get(&admitted),
            Some(&Position { place: road })
        );
    }

    /// A handle of the wrong namespace kind is a runtime mismatch. The
    /// cross-namespace canonical case is a compile error instead:
    /// `Ref::<EntityId>::Existing(subject_id)` does not compile, because
    /// `SubjectId`, `EntityId`, and `EdgeId` are distinct newtypes with no
    /// conversion between them and no public `Uuid` constructor.
    #[test]
    fn a_draft_handle_of_the_wrong_kind_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let before = kernel.snapshot().unwrap();
        let commits_before = kernel.journal.commit_count();

        let error = kernel
            .submit(
                command(
                    &before,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    admit(patch_of(vec![
                        entity("rhythm-tithe", "The Rhythm Tithe", EntityKind::Resource),
                        institution(
                            "rhythm-authority",
                            "The Rhythm Authority",
                            Some(Ref::Draft(DraftHandle::new("rhythm-tithe"))),
                        ),
                    ])),
                ),
                &auth_principal(owner()),
            )
            .unwrap_err();

        let KernelError::PatchRejected(mismatches) = error else {
            panic!("expected a rejected patch");
        };
        assert_eq!(
            mismatches,
            vec![Mismatch::WrongKind {
                site: at("rhythm-authority"),
                referent: RefName::Entity(Ref::Draft(draft("rhythm-tithe"))),
                expected: PLACE,
                actual: RefKind::Entity(EntityKind::Resource),
            }]
        );
        assert_eq!(kernel.journal.commit_count(), commits_before);
        assert_eq!(kernel.snapshot().unwrap(), before);
    }

    /// Rejection carries the complete failed-check set, not the first failure,
    /// and repairing exactly that set is enough to commit.
    #[test]
    fn rejection_returns_every_failed_check_and_repairing_exactly_those_commits() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let before = kernel.snapshot().unwrap();

        let broken = patch_of(vec![
            entity("rhythm-road", "The Rhythm Road", EntityKind::Place),
            entity("rhythm-road", "  ", EntityKind::Place),
            institution(
                "rhythm-authority",
                "The Rhythm Authority",
                Some(Ref::Draft(DraftHandle::new("cavity-yard"))),
            ),
            Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new("late-arrival"),
                label: "A Late Arrival".into(),
                kind: SubjectKind::Person,
                controller: NewController::Human {
                    principal: player(),
                },
                affordances: BTreeSet::from([AffordanceKind::Speak]),
                position: None,
            }),
        ]);
        let error = kernel
            .submit(
                command(
                    &before,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    admit(broken),
                ),
                &auth_principal(owner()),
            )
            .unwrap_err();
        let KernelError::PatchRejected(mismatches) = error else {
            panic!("expected a rejected patch");
        };
        let mut expected = vec![
            Mismatch::DuplicateHandle {
                handle: DraftHandle::new("rhythm-road"),
            },
            Mismatch::EmptyLabel {
                handle: DraftHandle::new("rhythm-road"),
            },
            Mismatch::UnresolvedDraft {
                site: at("rhythm-authority"),
                referent: draft("cavity-yard"),
                expected: PLACE,
            },
            Mismatch::UnadmittedController {
                handle: DraftHandle::new("late-arrival"),
            },
        ];
        expected.sort();
        assert_eq!(mismatches, expected);
        assert_eq!(kernel.journal.commit_count(), 1);

        let repaired = patch_of(vec![
            entity("rhythm-road", "The Rhythm Road", EntityKind::Place),
            entity("cavity-yard", "The Cavity Yard", EntityKind::Place),
            institution(
                "rhythm-authority",
                "The Rhythm Authority",
                Some(Ref::Draft(DraftHandle::new("cavity-yard"))),
            ),
            Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new("late-arrival"),
                label: "A Late Arrival".into(),
                kind: SubjectKind::Person,
                controller: NewController::NarrativePersona,
                affordances: BTreeSet::from([AffordanceKind::Speak]),
                position: None,
            }),
        ]);
        let receipt = submit_owner(&mut kernel, &before, admit(repaired));
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
        assert_eq!(kernel.state.entities.len(), 2);
    }

    #[test]
    fn an_empty_patch_is_no_canonical_change() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let before = kernel.snapshot().unwrap();

        let error = kernel
            .submit(
                command(
                    &before,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    admit(patch_of(Vec::new())),
                ),
                &auth_principal(owner()),
            )
            .unwrap_err();
        let KernelError::PatchRejected(mismatches) = error else {
            panic!("expected a rejected patch");
        };
        assert_eq!(mismatches, vec![Mismatch::NoCanonicalChange]);
        assert_eq!(kernel.snapshot().unwrap(), before);
        assert_eq!(kernel.journal.commit_count(), 1);
    }

    /// Active admits the four operations and nothing else, so it never mints a
    /// canonical ID.
    #[test]
    fn an_active_patch_that_declares_anything_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let active = activate(&mut kernel);
        let commits_before = kernel.journal.commit_count();
        let declaring = patch_of(vec![entity(
            "rhythm-road",
            "The Rhythm Road",
            EntityKind::Place,
        )]);
        let bearing_evidence = WorldPatch {
            declarations: Vec::new(),
            operations: Vec::new(),
            evidence: vec![EvidenceRef("run-115".into())],
        };

        for patch in [declaring, bearing_evidence] {
            let error = kernel
                .submit(
                    command(
                        &active,
                        CommandId::new(),
                        CallerId::Principal(owner()),
                        admit(patch),
                    ),
                    &auth_principal(owner()),
                )
                .unwrap_err();
            assert!(matches!(
                error,
                KernelError::WrongPhase {
                    expected: WorldPhase::Draft,
                    actual: WorldPhase::Active,
                }
            ));
        }
        assert_eq!(kernel.journal.commit_count(), commits_before);
        assert_eq!(kernel.snapshot().unwrap(), active);
        assert!(kernel.state.entities.is_empty());
    }

    /// Three places whose containers form a cycle name themselves, and the
    /// unrelated empty label joins the same complete set.
    #[test]
    fn a_containment_cycle_is_rejected_with_the_complete_set() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let before = kernel.snapshot().unwrap();

        let error = kernel
            .submit(
                command(
                    &before,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    admit(patch_of(vec![
                        contained("yard", "The Cavity Yard", "road"),
                        contained("road", "The Rhythm Road", "gate"),
                        contained("gate", "The Rain Gate", "yard"),
                        entity("orphan", "   ", EntityKind::Place),
                    ])),
                ),
                &auth_principal(owner()),
            )
            .unwrap_err();
        let KernelError::PatchRejected(mismatches) = error else {
            panic!("expected a rejected patch");
        };
        let mut expected = vec![
            Mismatch::ContainmentCycle {
                referent: draft("yard"),
            },
            Mismatch::ContainmentCycle {
                referent: draft("road"),
            },
            Mismatch::ContainmentCycle {
                referent: draft("gate"),
            },
            Mismatch::EmptyLabel {
                handle: draft("orphan"),
            },
        ];
        expected.sort();
        assert_eq!(mismatches, expected);
        assert_eq!(kernel.snapshot().unwrap(), before);
        assert!(kernel.state.entities.is_empty());
    }

    #[test]
    fn a_route_endpoint_that_is_not_a_place_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let before = kernel.snapshot().unwrap();

        let error = kernel
            .submit(
                command(
                    &before,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    admit(patch_of(vec![
                        entity("yard", "The Cavity Yard", EntityKind::Place),
                        entity("tithe", "The Rhythm Tithe", EntityKind::Resource),
                        route("ramp", "The Yard Ramp", "yard", "tithe", 12),
                    ])),
                ),
                &auth_principal(owner()),
            )
            .unwrap_err();
        let KernelError::PatchRejected(mismatches) = error else {
            panic!("expected a rejected patch");
        };
        assert_eq!(
            mismatches,
            vec![Mismatch::WrongKind {
                site: at("ramp"),
                referent: RefName::Entity(Ref::Draft(draft("tithe"))),
                expected: PLACE,
                actual: RefKind::Entity(EntityKind::Resource),
            }]
        );
        assert!(kernel.state.edges.is_empty());
        assert_eq!(kernel.snapshot().unwrap(), before);
    }

    /// One name for one failed check, whether the cost arrives in a declaration
    /// or in an operation.
    #[test]
    fn an_invalid_route_cost_is_rejected_from_declaration_and_from_alter_cost() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let topology = admit_topology(&mut kernel);
        let before = kernel.snapshot().unwrap();

        let error = kernel
            .submit(
                command(
                    &before,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    admit(WorldPatch {
                        declarations: vec![Declaration::Route(RouteDeclaration {
                            handle: draft("free-ride"),
                            label: "The Free Ride".into(),
                            from: Ref::Existing(topology.yard),
                            to: Ref::Existing(topology.road),
                            access: AccessKind::Public,
                            cost: Cost(0),
                        })],
                        operations: vec![ComponentOp::AlterCost {
                            route: Ref::Existing(topology.ramp),
                            cost: Cost(MAX_ROUTE_COST + 1),
                        }],
                        evidence: Vec::new(),
                    }),
                ),
                &auth_principal(owner()),
            )
            .unwrap_err();
        let KernelError::PatchRejected(mismatches) = error else {
            panic!("expected a rejected patch");
        };
        let mut expected = vec![
            Mismatch::InvalidCost {
                site: at("free-ride"),
            },
            Mismatch::InvalidCost {
                site: Site::Operation(0),
            },
        ];
        expected.sort();
        assert_eq!(mismatches, expected);
        assert_eq!(kernel.snapshot().unwrap(), before);
    }

    /// Each refusal has exactly one name: a closed route, a restricted one, and
    /// a route that does not start where the subject stands.
    #[test]
    fn relocate_without_an_open_public_route_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let topology = admit_topology(&mut kernel);
        let active = activate(&mut kernel);
        let commits_before = kernel.journal.commit_count();

        for (edge_id, expected) in [
            (topology.shutter, Mismatch::RouteClosed { operation: 0 }),
            (
                topology.toll,
                Mismatch::RouteAccessRestricted { operation: 0 },
            ),
            (topology.span, Mismatch::SubjectNotAtOrigin { operation: 0 }),
        ] {
            let error = kernel
                .submit(
                    command(
                        &active,
                        CommandId::new(),
                        CallerId::Principal(owner()),
                        admit(operations_of(vec![ComponentOp::Relocate {
                            subject: Ref::Existing(topology.walker),
                            via: Ref::Existing(edge_id),
                        }])),
                    ),
                    &auth_principal(owner()),
                )
                .unwrap_err();
            let KernelError::PatchRejected(mismatches) = error else {
                panic!("expected a rejected patch");
            };
            assert_eq!(mismatches, vec![expected]);
        }
        assert_eq!(kernel.journal.commit_count(), commits_before);
        assert_eq!(kernel.snapshot().unwrap(), active);
        assert_eq!(
            kernel.state.positions.get(&topology.walker),
            Some(&Position {
                place: topology.yard
            })
        );
    }

    #[test]
    fn relocate_moves_the_subject_and_nothing_else() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let topology = admit_topology(&mut kernel);
        let active = activate(&mut kernel);
        let entities_before = kernel.state.entities.clone();
        let edges_before = kernel.state.edges.clone();

        let receipt = submit_owner(
            &mut kernel,
            &active,
            admit(operations_of(vec![ComponentOp::Relocate {
                subject: Ref::Existing(topology.walker),
                via: Ref::Existing(topology.ramp),
            }])),
        );
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
        assert_eq!(
            kernel.state.positions.get(&topology.walker),
            Some(&Position {
                place: topology.road
            })
        );
        assert_eq!(kernel.state.entities, entities_before);
        assert_eq!(kernel.state.edges, edges_before);
        let after = kernel.snapshot().unwrap();
        assert_eq!(
            after
                .subjects
                .iter()
                .find(|subject| subject.id == topology.walker)
                .expect("the walker is in the snapshot")
                .position,
            Some(topology.road)
        );
    }

    #[test]
    fn restart_replay_after_a_relocate_is_exact() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "Kharad"),
            &auth_principal(owner()),
        )
        .unwrap();
        let topology = admit_topology(&mut kernel);
        let active = activate(&mut kernel);
        let envelope = command(
            &active,
            CommandId::new(),
            CallerId::Principal(owner()),
            admit(operations_of(vec![ComponentOp::Relocate {
                subject: Ref::Existing(topology.walker),
                via: Ref::Existing(topology.ramp),
            }])),
        );
        kernel
            .submit(envelope.clone(), &auth_principal(owner()))
            .unwrap();
        let committed = kernel.snapshot().unwrap();
        let world_id = committed.world_id;
        drop(kernel);

        let mut reopened = WorldKernel::open(&path, world_id).unwrap();
        assert_eq!(reopened.snapshot().unwrap(), committed);
        assert_eq!(
            reopened.state.positions.get(&topology.walker),
            Some(&Position {
                place: topology.road
            })
        );
        assert!(matches!(
            reopened.submit(envelope, &auth_principal(owner())).unwrap(),
            SubmitReceipt::AlreadyApplied(_)
        ));
    }

    #[test]
    fn the_snapshot_exposes_places_routes_and_positions_deterministically() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let topology = admit_topology(&mut kernel);
        let snapshot = kernel.snapshot().unwrap();
        assert_eq!(snapshot, kernel.snapshot().unwrap());

        assert_eq!(snapshot.places.len(), 3);
        let place_ids: Vec<_> = snapshot.places.iter().map(|place| place.id).collect();
        let mut sorted_places = place_ids.clone();
        sorted_places.sort();
        assert_eq!(place_ids, sorted_places);
        assert!(place_ids.contains(&topology.gate));

        assert_eq!(snapshot.routes.len(), 4);
        let route_ids: Vec<_> = snapshot.routes.iter().map(|route| route.id).collect();
        let mut sorted_routes = route_ids.clone();
        sorted_routes.sort();
        assert_eq!(route_ids, sorted_routes);
        let shutter = snapshot
            .routes
            .iter()
            .find(|route| route.id == topology.shutter)
            .expect("the shutter is in the snapshot");
        assert!(!shutter.open);
        assert_eq!(shutter.cost, Cost(5));
        assert_eq!(shutter.from, topology.yard);

        for subject in &snapshot.subjects {
            assert_eq!(
                subject.position,
                kernel
                    .state
                    .positions
                    .get(&subject.id)
                    .map(|position| position.place)
            );
        }
    }

    #[test]
    fn admit_patch_from_a_non_owner_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let before = kernel.snapshot().unwrap();
        let commits_before = kernel.journal.commit_count();

        let error = kernel
            .submit(
                command(
                    &before,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    admit(patch_of(vec![entity(
                        "rhythm-road",
                        "The Rhythm Road",
                        EntityKind::Place,
                    )])),
                ),
                &auth_principal(player()),
            )
            .unwrap_err();
        assert!(matches!(error, KernelError::Unauthorized));
        assert_eq!(kernel.journal.commit_count(), commits_before);
        assert_eq!(kernel.snapshot().unwrap(), before);
    }

    #[test]
    fn restart_replay_after_a_committed_patch_is_exact() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "Kharad"),
            &auth_principal(owner()),
        )
        .unwrap();
        let before = kernel.snapshot().unwrap();
        let envelope = command(
            &before,
            CommandId::new(),
            CallerId::Principal(owner()),
            admit(patch_of(vec![
                entity("rhythm-road", "The Rhythm Road", EntityKind::Place),
                institution(
                    "rhythm-authority",
                    "The Rhythm Authority",
                    Some(Ref::Draft(DraftHandle::new("rhythm-road"))),
                ),
            ])),
        );
        kernel
            .submit(envelope.clone(), &auth_principal(owner()))
            .unwrap();
        let committed = kernel.snapshot().unwrap();
        let world_id = committed.world_id;
        drop(kernel);

        let mut reopened = WorldKernel::open(&path, world_id).unwrap();
        assert_eq!(reopened.snapshot().unwrap(), committed);
        assert!(matches!(
            reopened.submit(envelope, &auth_principal(owner())).unwrap(),
            SubmitReceipt::AlreadyApplied(_)
        ));
    }

    /// One allocator, a pure function of `(world_id, command_id, handle)`: the
    /// same handle text under two commands gets two IDs, and the same command
    /// re-resolved gets the same ones.
    #[test]
    fn genesis_and_admit_patch_share_one_allocator() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let before = kernel.snapshot().unwrap();
        let genesis_player = before
            .subjects
            .iter()
            .find(|subject| subject.label == "The Player")
            .expect("the genesis player subject")
            .id;

        let patch = patch_of(vec![Declaration::Subject(SubjectDeclaration {
            handle: DraftHandle::new("player"),
            label: "Another Player".into(),
            kind: SubjectKind::Person,
            controller: NewController::NarrativePersona,
            affordances: BTreeSet::from([AffordanceKind::Speak]),
            position: None,
        })]);
        submit_owner(&mut kernel, &before, admit(patch.clone()));
        let admitted = *kernel
            .state
            .subjects
            .iter()
            .find(|(_, subject)| subject.label == "Another Player")
            .expect("the admitted subject")
            .0;
        assert_ne!(admitted, genesis_player);

        let command_id = CommandId::new();
        let first = resolve_patch(&kernel.state, command_id, &patch).unwrap();
        let second = resolve_patch(&kernel.state, command_id, &patch).unwrap();
        assert_eq!(first, second);
    }

    /// The durable half of "a rejected patch mints nothing": the store on disk,
    /// not just the in-memory head, carries no row and no byte of any ID the
    /// rejected patch would have allocated.
    #[test]
    fn a_rejected_patch_leaves_no_row_and_no_id_in_the_store() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "Kharad"),
            &auth_principal(owner()),
        )
        .unwrap();
        let before = kernel.snapshot().unwrap();
        let command_id = CommandId::new();
        let handle = DraftHandle::new("rhythm-authority");

        let error = kernel
            .submit(
                command(
                    &before,
                    command_id,
                    CallerId::Principal(owner()),
                    admit(patch_of(vec![institution(
                        "rhythm-authority",
                        "The Rhythm Authority",
                        Some(Ref::Draft(DraftHandle::new("kharad-rhythm-road"))),
                    )])),
                ),
                &auth_principal(owner()),
            )
            .unwrap_err();
        assert!(matches!(error, KernelError::PatchRejected(_)));

        // Every ID the patch would have allocated, had resolution closed.
        let would_be = [
            derive_id(
                SUBJECT_NAMESPACE,
                before.world_id,
                command_id,
                &handle,
                None,
            ),
            derive_id(
                CONTROLLER_NAMESPACE,
                before.world_id,
                command_id,
                &handle,
                None,
            ),
            derive_id(
                AFFORDANCE_NAMESPACE,
                before.world_id,
                command_id,
                &handle,
                Some("speak"),
            ),
        ];

        drop(kernel);
        let raw = std::fs::read(&path).unwrap();
        for id in would_be {
            assert!(
                !raw.windows(16).any(|window| window == id.as_bytes()),
                "a rejected patch left {id} in the store"
            );
        }

        let reopened = WorldKernel::open(&path, before.world_id).unwrap();
        assert_eq!(reopened.snapshot().unwrap(), before);
        assert_eq!(reopened.journal.commit_count(), 1);
        assert!(reopened.state.entities.is_empty());
        assert!(
            !reopened
                .state
                .subjects
                .contains_key(&SubjectId(would_be[0]))
        );
    }

    /// The `PatchAdmitted` arm is not a trusting applier. A forged effect, a
    /// replayed one, a non-owner caller, and the Active phase each die inside
    /// `apply_effect`, without the journal's reduce-equality check running.
    #[test]
    fn apply_effect_rejects_a_forged_stale_unauthorized_or_active_patch_effect() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let before = kernel.snapshot().unwrap();
        let patch = patch_of(vec![
            entity("rhythm-road", "The Rhythm Road", EntityKind::Place),
            institution(
                "rhythm-authority",
                "The Rhythm Authority",
                Some(Ref::Draft(DraftHandle::new("rhythm-road"))),
            ),
        ]);
        let resolved = resolve_patch(&kernel.state, CommandId::new(), &patch).unwrap();
        let honest = super::super::WorldEffect::PatchAdmitted {
            resolved: resolved.clone(),
        };

        // Forged: the position names a place no partition holds, and the effect
        // no longer carries the entity that would have created it.
        let mut forged = resolved.clone();
        forged.entities.clear();
        forged.subjects[0].position = Some(Position {
            place: super::super::EntityId::issue(),
        });
        let mut candidate = kernel.state.clone();
        let forged_error = super::super::apply_effect(
            &mut candidate,
            &CallerId::Principal(owner()),
            &super::super::WorldEffect::PatchAdmitted { resolved: forged },
        )
        .unwrap_err();
        assert!(matches!(forged_error, KernelError::Invariant(_)));

        // Stale: the honest effect applied twice collides on its own IDs.
        let mut candidate = kernel.state.clone();
        super::super::apply_effect(&mut candidate, &CallerId::Principal(owner()), &honest).unwrap();
        let stale_error =
            super::super::apply_effect(&mut candidate, &CallerId::Principal(owner()), &honest)
                .unwrap_err();
        assert!(matches!(stale_error, KernelError::Invariant(_)));

        // A non-owner caller never reaches admission.
        let mut candidate = kernel.state.clone();
        let unauthorized =
            super::super::apply_effect(&mut candidate, &CallerId::Principal(player()), &honest)
                .unwrap_err();
        assert!(matches!(unauthorized, KernelError::Invariant(_)));
        assert_eq!(candidate, kernel.state);

        // And the Active phase is refused before any partition is touched.
        let active = activate(&mut kernel);
        assert_eq!(active.revision, before.revision + 3);
        let mut candidate = kernel.state.clone();
        let wrong_phase =
            super::super::apply_effect(&mut candidate, &CallerId::Principal(owner()), &honest)
                .unwrap_err();
        assert!(matches!(wrong_phase, KernelError::Invariant(_)));
        assert_eq!(candidate, kernel.state);
    }

    /// Soul falsification: `RefKind` is never persisted, and it could not be.
    /// `Subject(Option<SubjectKind>)` under `#[serde(tag = "namespace")]` is an
    /// internally tagged newtype variant over a non-map, which serde refuses at
    /// runtime. The derives are unexercised weight, and the deviation is safe
    /// only because nothing writes a `RefKind`.
    #[test]
    fn soul_ref_kind_is_not_a_serializable_shape() {
        assert!(serde_json::to_value(RefKind::Entity(EntityKind::Place)).is_ok());
        assert!(serde_json::to_value(ANY_SUBJECT).is_err());
        assert!(serde_json::to_value(RefKind::Subject(Some(SubjectKind::Person))).is_err());
    }

    /// Soul falsification: the three pass-2 `Mismatch` variants that no landed
    /// test names are each reachable from a real patch.
    #[test]
    fn soul_self_loop_unplaced_subject_and_no_effect_are_each_reachable() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let before = kernel.snapshot().unwrap();

        // A route whose two endpoints resolve to one place.
        let error = kernel
            .submit(
                command(
                    &before,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    admit(patch_of(vec![
                        entity("hollow", "The Hollow", EntityKind::Place),
                        route("noose", "The Noose", "hollow", "hollow", 4),
                    ])),
                ),
                &auth_principal(owner()),
            )
            .unwrap_err();
        let KernelError::PatchRejected(mismatches) = error else {
            panic!("expected a rejected patch");
        };
        assert_eq!(
            mismatches,
            vec![Mismatch::RouteSelfLoop {
                referent: draft("noose")
            }]
        );

        let topology = admit_topology(&mut kernel);
        let active = activate(&mut kernel);
        let commits_before = kernel.journal.commit_count();
        let unplaced = *kernel
            .state
            .subjects
            .iter()
            .find(|(subject_id, _)| !kernel.state.positions.contains_key(*subject_id))
            .expect("a genesis subject was declared without a position")
            .0;

        // Relocating a subject that stands nowhere.
        let error = kernel
            .submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    admit(operations_of(vec![ComponentOp::Relocate {
                        subject: Ref::Existing(unplaced),
                        via: Ref::Existing(topology.ramp),
                    }])),
                ),
                &auth_principal(owner()),
            )
            .unwrap_err();
        let KernelError::PatchRejected(mismatches) = error else {
            panic!("expected a rejected patch");
        };
        assert_eq!(mismatches, vec![Mismatch::UnplacedSubject { operation: 0 }]);

        // Three operations that each change nothing: the ramp is already open,
        // the shutter already closed, and the ramp already costs twelve.
        let error = kernel
            .submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    admit(operations_of(vec![
                        ComponentOp::OpenRoute {
                            route: Ref::Existing(topology.ramp),
                        },
                        ComponentOp::CloseRoute {
                            route: Ref::Existing(topology.shutter),
                        },
                        ComponentOp::AlterCost {
                            route: Ref::Existing(topology.ramp),
                            cost: Cost(12),
                        },
                    ])),
                ),
                &auth_principal(owner()),
            )
            .unwrap_err();
        let KernelError::PatchRejected(mismatches) = error else {
            panic!("expected a rejected patch");
        };
        assert_eq!(
            mismatches,
            vec![
                Mismatch::NoOperationEffect { operation: 0 },
                Mismatch::NoOperationEffect { operation: 1 },
                Mismatch::NoOperationEffect { operation: 2 },
            ]
        );
        assert_eq!(kernel.snapshot().unwrap(), active);
        assert_eq!(kernel.journal.commit_count(), commits_before);
    }

    /// Soul falsification: `admit_resolved` re-derives every relocate
    /// precondition, so a forged effect naming a route the subject does not
    /// stand on, a closed route, or a restricted one dies at apply time.
    #[test]
    fn soul_a_forged_relocate_effect_is_refused_at_apply_time() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let topology = admit_topology(&mut kernel);
        activate(&mut kernel);
        for edge_id in [topology.span, topology.shutter, topology.toll] {
            let forged = super::super::WorldEffect::PatchAdmitted {
                resolved: ResolvedPatch {
                    subjects: Vec::new(),
                    entities: Vec::new(),
                    routes: Vec::new(),
                    operations: vec![ResolvedOp::Relocate {
                        subject_id: topology.walker,
                        edge_id,
                    }],
                    evidence: Vec::new(),
                },
            };
            let mut candidate = kernel.state.clone();
            let error = super::super::apply_effect(
                &mut candidate,
                &CallerId::Principal(owner()),
                &forged,
            )
            .unwrap_err();
            assert!(matches!(error, KernelError::Invariant(_)));
            assert_eq!(candidate, kernel.state);
        }
    }
}
