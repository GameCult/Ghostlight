//! Closed-patch resolution: the one owner of draft handles, reference kinds, and
//! canonical ID allocation for every admission lane.
//!
//! A patch names structure two ways and no other way: an exact canonical ID that
//! already keys a partition, or a draft handle declared in the same patch.
//! Resolution accumulates the complete mismatch set first and allocates only
//! after that set is empty, so a rejected patch never mints an ID.

use super::{
    AffordanceId, CommandId, ControllerAssignment, ControllerId, EdgeId, EntityId, NewController,
    SubjectId, SubjectKind, SubjectState, WorldId,
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

/// A conserved count. Unsigned because a holding can never be negative: a signed
/// type would make "owes 3 iron" representable and push the real check to runtime
/// anyway. Subtraction is `checked_sub` and its failure is `InsufficientCustody`;
/// addition is `checked_add` and its failure is `QuantityOverflow`. No saturating
/// arithmetic anywhere, because a silent clamp is a silent conservation break.
/// There is no maximum: nothing sums quantities across resources, so the honest
/// bound is the type's. There is no unit, because a unit is where prices enter.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct Quantity(pub(crate) u64);

/// What a subject depends on. A dependency is a bare relation: no magnitude, no
/// kind beyond its target namespace, no clock, no cost. The variant selects the
/// resolver, so the target kind is carried by the type rather than checked after
/// the fact.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "target", content = "id", rename_all = "snake_case")]
pub(crate) enum DependencyTarget {
    Resource(EntityId),
    Route(EdgeId),
    Subject(SubjectId),
}

/// The proposal-time twin of [`DependencyTarget`], carrying references rather
/// than canonical IDs.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "target", content = "ref", rename_all = "snake_case")]
pub(crate) enum DependencyRef {
    Resource(Ref<EntityId>),
    Route(Ref<EdgeId>),
    Subject(Ref<SubjectId>),
}

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
///
/// It is one predicate serving three readers: the resolution index, `Mismatch`,
/// and an affordance's `RoleSpec`, which persists inside the catalog and so
/// requires the serde derives. `Affordance` names the catalog namespace, which
/// carries no components and can never be a role's target.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "namespace", content = "kind", rename_all = "snake_case")]
pub(crate) enum RefKind {
    Subject(Option<SubjectKind>),
    Entity(EntityKind),
    Edge(EdgeKind),
    Affordance,
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

/// A referent in any of the three referent namespaces, or in the catalog.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RefName {
    Subject(Ref<SubjectId>),
    Entity(Ref<EntityId>),
    Edge(Ref<EdgeId>),
    Affordance(Ref<AffordanceId>),
}

/// A world-declared affordance name. Canonical text, `[a-z][a-z0-9_]{0,47}`,
/// because it becomes a generated tool name and a tool name must be stable,
/// unique, and safe. The kernel carries it and branches on it nowhere; that is
/// what keeps genre out of the reducer.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct AffordanceKindName(pub(crate) String);

/// A named slot in one affordance, bound to a referent at invocation time.
/// Canonical text, `[a-z][a-z0-9_]{0,31}` — it becomes a tool parameter name.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct Role(pub(crate) String);

/// What a role must be bound to. `RefKind` is reused rather than mirrored: it
/// already spells subject, entity kind, and edge kind, so the binding kind-check
/// and the declaration kind-check are one predicate.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub(crate) struct RoleSpec {
    pub(crate) role: Role,
    pub(crate) kind: RefKind,
}

/// What must already be true of committed state before an invocation is
/// admitted. Only the three whose components exist: `Authorized`, `Knows`,
/// `CanReach`, and `Committed` land in the pass that adds `Authority`,
/// `Knowledge`, `Channel`, and `Commitment`. A variant whose only behaviour
/// would be to be refused is a placeholder wearing a check.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "precondition", rename_all = "snake_case")]
pub(crate) enum Precondition {
    /// The acting subject's Position names the place bound to `at`.
    Present { at: Role },
    /// A path exists from the actor's place to the place bound to `to`, over
    /// open public routes, with summed cost at most `within`.
    Reachable { to: Role, within: Cost },
    /// The acting subject's own holding of the resource bound to `resource` is
    /// at least `at_least`.
    Holds { resource: Role, at_least: Quantity },
}

/// Exactly the operations an affordance may propose. Nine, not the resolver's
/// ten: `Admit` is absent because minting quantity requires an `EvidenceRef` and
/// an invocation carries no evidence list, so admitting it here would be a
/// second creation path beside the single evidenced one.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ComponentOpKind {
    Relocate,
    OpenRoute,
    CloseRoute,
    AlterCost,
    Transfer,
    Transform,
    Consume,
    Bind,
    Release,
}

/// The referent shape of one operation: the single source of both the
/// declaration check and the lowering.
pub(super) enum RoleKindRule {
    Exact(RefKind),
    AnyDependencyTarget,
}

/// Which magnitude, if any, an operation carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BoundsDimension {
    None,
    Quantity,
    Cost,
}

impl ComponentOpKind {
    pub(super) fn arity(self) -> Vec<RoleKindRule> {
        let subject = RoleKindRule::Exact(ANY_SUBJECT);
        let route = || RoleKindRule::Exact(ROUTE);
        let resource = || RoleKindRule::Exact(RefKind::Entity(EntityKind::Resource));
        match self {
            Self::Relocate => vec![subject, route()],
            Self::OpenRoute | Self::CloseRoute | Self::AlterCost => vec![route()],
            Self::Transfer => vec![subject, RoleKindRule::Exact(ANY_SUBJECT), resource()],
            Self::Transform => vec![subject, resource(), resource()],
            Self::Consume => vec![subject, resource()],
            Self::Bind | Self::Release => vec![subject, RoleKindRule::AnyDependencyTarget],
        }
    }

    pub(super) fn dimension(self) -> BoundsDimension {
        match self {
            Self::Relocate | Self::OpenRoute | Self::CloseRoute | Self::Bind | Self::Release => {
                BoundsDimension::None
            }
            Self::Transfer | Self::Transform | Self::Consume => BoundsDimension::Quantity,
            Self::AlterCost => BoundsDimension::Cost,
        }
    }
}

/// Whether a declared role kind can serve a position governed by this rule.
pub(super) fn role_kind_fits(rule: &RoleKindRule, declared: RefKind) -> bool {
    match rule {
        RoleKindRule::Exact(RefKind::Subject(_)) => matches!(declared, RefKind::Subject(_)),
        RoleKindRule::Exact(expected) => *expected == declared,
        RoleKindRule::AnyDependencyTarget => matches!(
            declared,
            RefKind::Subject(_) | RefKind::Entity(EntityKind::Resource) | RefKind::Edge(_)
        ),
    }
}

/// The magnitude ceiling for one slot. A closed enum matched to the operation's
/// dimension rather than a struct of optional ceilings: an `Option` ceiling that
/// is `None` is not a ceiling, and a struct admits the nonsense of a cost
/// ceiling on a `Transfer`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "bound", content = "max", rename_all = "snake_case")]
pub(crate) enum Bounds {
    None,
    Quantity(Quantity),
    Cost(Cost),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct EffectSlot {
    pub(crate) op_kind: ComponentOpKind,
    /// Positional: exactly the referents `op_kind` takes, in `arity()` order.
    pub(crate) roles: Vec<Role>,
    pub(crate) bounds: Bounds,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutcomeBand {
    /// At least 1. A zero-weight band is a branch that can never be selected.
    pub(crate) weight: u32,
    /// Indices into `effect_slots`, strictly increasing. Empty is legal and is
    /// how a world expresses "the attempt does nothing".
    pub(crate) effects: Vec<usize>,
}

/// One catalog entry: what an affordance *is*. Who may use it is
/// `affordance_grants` and nothing else, so an entry carries no audience.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Affordance {
    pub(crate) kind: AffordanceKindName,
    /// Ordered, so the generated tool's parameters are stable across builds.
    pub(crate) roles: Vec<RoleSpec>,
    pub(crate) preconditions: Vec<Precondition>,
    pub(crate) effect_slots: Vec<EffectSlot>,
    pub(crate) outcome_bands: Vec<OutcomeBand>,
    /// Every invocation of this entry carries exactly one utterance. The
    /// kernel's only behaviour is to record it in the event; any world may
    /// declare a speaking affordance, so this is not a Speak special case.
    pub(crate) carries_speech: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct AffordanceDeclaration {
    pub(crate) handle: DraftHandle,
    pub(crate) kind: AffordanceKindName,
    pub(crate) roles: Vec<RoleSpec>,
    pub(crate) preconditions: Vec<Precondition>,
    pub(crate) effect_slots: Vec<EffectSlot>,
    pub(crate) outcome_bands: Vec<OutcomeBand>,
    pub(crate) carries_speech: bool,
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

impl EvidenceRef {
    /// Gated because no `AdmitPatch` ingress exists yet: every caller that
    /// builds a patch today is a test. It loses the gate in the same commit as
    /// the first production patch author.
    #[cfg(test)]
    pub(super) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

/// `position` is the subject's presence: one place it stands in. A subject
/// declared without one is unplaced until a later pass gives placement an owner.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubjectDeclaration {
    pub(crate) handle: DraftHandle,
    pub(crate) label: String,
    pub(crate) kind: SubjectKind,
    pub(crate) controller: NewController,
    /// The catalog entries this subject may exercise. Every entry is a
    /// reference, so a genesis patch declares a catalog and grants from it
    /// atomically.
    pub(crate) affordances: BTreeSet<Ref<AffordanceId>>,
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
    Affordance(AffordanceDeclaration),
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
    Transfer {
        from: Ref<SubjectId>,
        to: Ref<SubjectId>,
        resource: Ref<EntityId>,
        qty: Quantity,
    },
    /// One-for-one relabel: the same `qty` leaves `from_resource` and arrives in
    /// `into_resource`, held by the same subject. A ratio would be a recipe, and
    /// recipes belong to the consumer that owns production; the op shape makes a
    /// non-conserving transform unrepresentable rather than merely checkable. A
    /// world that needs lossy conversion writes `Transform` plus `Consume`, and
    /// both terms then appear in the ledger.
    Transform {
        holder: Ref<SubjectId>,
        from_resource: Ref<EntityId>,
        into_resource: Ref<EntityId>,
        qty: Quantity,
    },
    Consume {
        holder: Ref<SubjectId>,
        resource: Ref<EntityId>,
        qty: Quantity,
    },
    /// The only creation path for quantity, in every lane including genesis. Its
    /// `evidence` must appear in the same patch's `evidence` list, which makes
    /// creation attributable; the commit chain then makes it tamper-evident.
    Admit {
        holder: Ref<SubjectId>,
        resource: Ref<EntityId>,
        qty: Quantity,
        evidence: EvidenceRef,
    },
    Bind {
        subject: Ref<SubjectId>,
        target: DependencyRef,
    },
    Release {
        subject: Ref<SubjectId>,
        target: DependencyRef,
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
    /// The holder does not hold enough. Absence is zero, so "holds none at all"
    /// is this variant at the boundary rather than a second name.
    InsufficientCustody {
        operation: usize,
    },
    /// A custody operation naming `Quantity(0)`: it moves nothing.
    ZeroQuantity {
        operation: usize,
    },
    /// A credit that would leave a holding above `u64::MAX`.
    QuantityOverflow {
        operation: usize,
    },
    /// An `Admit` whose `EvidenceRef` is not listed in this patch's `evidence`.
    AdmitWithoutEvidence {
        operation: usize,
    },
    /// A `Bind` or `Release` naming the acting subject as its own target.
    SelfDependency {
        operation: usize,
    },
    /// The candidate ledger does not balance for this resource: the total after
    /// the patch is not the total before plus what was admitted and gained, less
    /// what was consumed and spent.
    CustodyNotConserved {
        resource: RefName,
    },
    /// `kind` is not `[a-z][a-z0-9_]{0,47}`, or a `Role` is not
    /// `[a-z][a-z0-9_]{0,31}`. Tool and parameter names must be safe.
    InvalidAffordanceName {
        handle: DraftHandle,
    },
    /// Two entries in the candidate graph share one kind name; the generated
    /// tool catalog would have two tools with one name.
    DuplicateAffordanceKind {
        handle: DraftHandle,
    },
    DuplicateRole {
        handle: DraftHandle,
        role: Role,
    },
    /// A precondition or a slot names a role the entry does not declare.
    UnknownRole {
        handle: DraftHandle,
        role: Role,
    },
    /// A declared role's `RefKind` cannot serve the position that reads it, or
    /// names a namespace no invocation target can bind.
    RoleKindUnfit {
        handle: DraftHandle,
        role: Role,
    },
    SlotRoleArity {
        handle: DraftHandle,
        slot: usize,
    },
    /// The slot's `Bounds` variant does not match its `op_kind`'s dimension, or
    /// the ceiling is zero, or a cost ceiling is outside `1..=MAX_ROUTE_COST`.
    SlotBoundMismatch {
        handle: DraftHandle,
        slot: usize,
    },
    NoOutcomeBand {
        handle: DraftHandle,
    },
    ZeroBandWeight {
        handle: DraftHandle,
        band: usize,
    },
    /// A band names a slot index the entry does not have.
    DanglingBandEffect {
        handle: DraftHandle,
        band: usize,
    },
    /// A band's `effects` are not strictly increasing.
    BandEffectsNotCanonical {
        handle: DraftHandle,
        band: usize,
    },
    /// No effect slots and no speech: the entry can never change anything.
    InertAffordance {
        handle: DraftHandle,
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
    pub(super) affordances: BTreeSet<AffordanceId>,
    pub(super) position: Option<Position>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct ResolvedEntity {
    pub(super) handle: DraftHandle,
    pub(super) entity_id: EntityId,
    pub(super) entity: EntityRecord,
}

/// The handle-to-ID binding for one catalog entry.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct ResolvedAffordance {
    pub(super) handle: DraftHandle,
    pub(super) affordance_id: AffordanceId,
    pub(super) affordance: Affordance,
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
pub(crate) enum ResolvedOp {
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
    Transfer {
        from: SubjectId,
        to: SubjectId,
        resource: EntityId,
        qty: Quantity,
    },
    Transform {
        holder: SubjectId,
        from_resource: EntityId,
        into_resource: EntityId,
        qty: Quantity,
    },
    Consume {
        holder: SubjectId,
        resource: EntityId,
        qty: Quantity,
    },
    Admit {
        holder: SubjectId,
        resource: EntityId,
        qty: Quantity,
        evidence: EvidenceRef,
    },
    Bind {
        subject: SubjectId,
        target: DependencyTarget,
    },
    Release {
        subject: SubjectId,
        target: DependencyTarget,
    },
}

/// One resource's movement across a whole patch. Accumulators are `u128` so an
/// intermediate total cannot overflow while a stored per-holder `Quantity` stays
/// `u64`.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(super) struct LedgerDelta {
    pub(super) before: u128,
    pub(super) after: u128,
    pub(super) admitted: u128,
    pub(super) consumed: u128,
    pub(super) gained: u128,
    pub(super) spent: u128,
}

/// The only statement of custody conservation, called by the resolver over the
/// candidate map and again by `admit_resolved` over the committed partitions.
/// Returns the first resource whose ledger does not balance, in key order, so
/// the verdict is deterministic.
///
/// `Transfer` contributes to no term: its correctness is *proven by* this
/// equation, because a transfer that moved a different amount out than in breaks
/// the total for that resource. `Transform` contributes the same `qty` to one
/// `spent` and one `gained`, so the one-for-one rule is this equation restricted
/// to a single operation. Nothing else states conservation anywhere.
pub(super) fn check_ledger<R: Ord + Clone>(deltas: &BTreeMap<R, LedgerDelta>) -> Option<R> {
    deltas
        .iter()
        .find(|(_, delta)| {
            let credited = delta
                .before
                .checked_add(delta.admitted)
                .and_then(|total| total.checked_add(delta.gained));
            let expected = credited
                .and_then(|total| total.checked_sub(delta.consumed))
                .and_then(|total| total.checked_sub(delta.spent));
            expected != Some(delta.after)
        })
        .map(|(resource, _)| resource.clone())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct ResolvedPatch {
    pub(super) subjects: Vec<ResolvedSubject>,
    pub(super) entities: Vec<ResolvedEntity>,
    pub(super) routes: Vec<ResolvedRoute>,
    pub(super) affordances: Vec<ResolvedAffordance>,
    pub(super) operations: Vec<ResolvedOp>,
    pub(super) evidence: Vec<EvidenceRef>,
}

impl ResolvedPatch {
    pub(super) fn declares_nothing(&self) -> bool {
        self.subjects.is_empty()
            && self.entities.is_empty()
            && self.routes.is_empty()
            && self.affordances.is_empty()
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

/// [`DependencyTarget`] over candidate keys, so a dependency on a structure this
/// patch declares is checked before any ID is minted.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum TargetKey {
    Resource(Key<EntityId>),
    Route(Key<EdgeId>),
    Subject(Key<SubjectId>),
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

pub(super) fn is_canonical_text(value: &str) -> bool {
    !value.trim().is_empty() && value.trim() == value
}

pub(super) fn is_valid_cost(cost: Cost) -> bool {
    (1..=MAX_ROUTE_COST).contains(&cost.0)
}

/// `[a-z][a-z0-9_]{0,max-1}`. Affordance kinds and roles become generated tool
/// and parameter names, so the alphabet is the safe one and the bound is stated
/// rather than assumed.
fn is_tool_name(value: &str, max: usize) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
        && value.len() <= max
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

/// The handle of the kernel-built Speak entry. It is indexed in the same draft
/// index as world-declared entries, so a genesis subject grants it with an
/// ordinary `Ref::Draft` and there is no second grant path.
pub(super) const KERNEL_SPEAK_HANDLE: &str = "ghostlight.affordance.speak";

/// The grant every genesis subject carries: a draft reference to the
/// kernel-built entry, resolved by the ordinary handle index. World ingress and
/// every fixture name this rather than repeating the handle literal.
pub(super) fn kernel_speak_grant() -> BTreeSet<Ref<AffordanceId>> {
    BTreeSet::from([Ref::Draft(DraftHandle::new(KERNEL_SPEAK_HANDLE))])
}

/// Zero preconditions, zero slots, one empty band, one utterance. Speech reaches
/// everyone because `Channel` does not exist yet; a reach check that always
/// passed would look like the invariant is enforced.
pub(super) fn kernel_speak_entry() -> Affordance {
    Affordance {
        kind: AffordanceKindName("speak".into()),
        roles: Vec::new(),
        preconditions: Vec::new(),
        effect_slots: Vec::new(),
        outcome_bands: vec![OutcomeBand {
            weight: 1,
            effects: Vec::new(),
        }],
        carries_speech: true,
    }
}

/// Whether a stored catalog entry would still be admitted by the declaration
/// validator. One validator, two readers: the resolver rejects a bad entry, and
/// journal recovery refuses a store that already holds one.
pub(super) fn entry_is_admissible(entry: &Affordance) -> bool {
    let mut mismatches = Vec::new();
    validate_affordance(
        &DraftHandle::new(""),
        &entry.kind,
        &entry.roles,
        &entry.preconditions,
        &entry.effect_slots,
        &entry.outcome_bands,
        entry.carries_speech,
        &mut mismatches,
    );
    mismatches.is_empty()
}

/// The catalog-entry resolver, beside `resolve_entity`/`_subject`/`_route`. An
/// entry carries no components, so there is nothing to check past namespace
/// agreement.
fn resolve_affordance(
    site: Site,
    reference: &Ref<AffordanceId>,
    index: &BTreeMap<DraftHandle, RefKind>,
    catalog: &BTreeMap<AffordanceId, Affordance>,
    mismatches: &mut Vec<Mismatch>,
) -> Option<Key<AffordanceId>> {
    match reference {
        Ref::Draft(named) => match index.get(named) {
            None => {
                mismatches.push(Mismatch::UnresolvedDraft {
                    site,
                    referent: named.clone(),
                    expected: RefKind::Affordance,
                });
                None
            }
            Some(kind) if *kind != RefKind::Affordance => {
                mismatches.push(Mismatch::WrongKind {
                    site,
                    referent: RefName::Affordance(reference.clone()),
                    expected: RefKind::Affordance,
                    actual: *kind,
                });
                None
            }
            Some(_) => Some(Key::Draft(named.clone())),
        },
        Ref::Existing(affordance_id) => {
            if catalog.contains_key(affordance_id) {
                Some(Key::Existing(*affordance_id))
            } else {
                mismatches.push(Mismatch::UnknownCanonical {
                    site,
                    expected: RefKind::Affordance,
                });
                None
            }
        }
    }
}

/// The one validator of a catalog entry, run per declared entry in declaration
/// order inside the resolver's accumulate-then-gate loop.
fn validate_affordance(
    handle: &DraftHandle,
    kind: &AffordanceKindName,
    roles: &[RoleSpec],
    preconditions: &[Precondition],
    effect_slots: &[EffectSlot],
    outcome_bands: &[OutcomeBand],
    carries_speech: bool,
    mismatches: &mut Vec<Mismatch>,
) {
    let site = || Site::Declaration(handle.clone());
    if !is_tool_name(&kind.0, 48) || roles.iter().any(|spec| !is_tool_name(&spec.role.0, 32)) {
        mismatches.push(Mismatch::InvalidAffordanceName {
            handle: handle.clone(),
        });
    }
    let mut declared: BTreeMap<Role, RefKind> = BTreeMap::new();
    for spec in roles {
        match declared.entry(spec.role.clone()) {
            Entry::Vacant(slot) => {
                slot.insert(spec.kind);
            }
            Entry::Occupied(_) => mismatches.push(Mismatch::DuplicateRole {
                handle: handle.clone(),
                role: spec.role.clone(),
            }),
        }
        // No `Target` names the catalog namespace, so a role of that kind could
        // never be bound.
        if spec.kind == RefKind::Affordance {
            mismatches.push(Mismatch::RoleKindUnfit {
                handle: handle.clone(),
                role: spec.role.clone(),
            });
        }
    }
    let require =
        |role: &Role, expected: RefKind, mismatches: &mut Vec<Mismatch>| match declared.get(role) {
            None => mismatches.push(Mismatch::UnknownRole {
                handle: handle.clone(),
                role: role.clone(),
            }),
            Some(actual) if *actual != expected => mismatches.push(Mismatch::RoleKindUnfit {
                handle: handle.clone(),
                role: role.clone(),
            }),
            Some(_) => {}
        };
    for precondition in preconditions {
        match precondition {
            Precondition::Present { at } => {
                require(at, RefKind::Entity(EntityKind::Place), mismatches);
            }
            Precondition::Reachable { to, within } => {
                require(to, RefKind::Entity(EntityKind::Place), mismatches);
                if !is_valid_cost(*within) {
                    mismatches.push(Mismatch::InvalidCost { site: site() });
                }
            }
            Precondition::Holds { resource, .. } => {
                require(resource, RefKind::Entity(EntityKind::Resource), mismatches);
            }
        }
    }
    for (index, slot) in effect_slots.iter().enumerate() {
        let arity = slot.op_kind.arity();
        if slot.roles.len() != arity.len() {
            mismatches.push(Mismatch::SlotRoleArity {
                handle: handle.clone(),
                slot: index,
            });
        }
        for (role, rule) in slot.roles.iter().zip(arity.iter()) {
            match declared.get(role) {
                None => mismatches.push(Mismatch::UnknownRole {
                    handle: handle.clone(),
                    role: role.clone(),
                }),
                Some(declared_kind) if !role_kind_fits(rule, *declared_kind) => {
                    mismatches.push(Mismatch::RoleKindUnfit {
                        handle: handle.clone(),
                        role: role.clone(),
                    });
                }
                Some(_) => {}
            }
        }
        let fits = match (slot.op_kind.dimension(), slot.bounds) {
            (BoundsDimension::None, Bounds::None) => true,
            (BoundsDimension::Quantity, Bounds::Quantity(ceiling)) => ceiling.0 >= 1,
            (BoundsDimension::Cost, Bounds::Cost(ceiling)) => is_valid_cost(ceiling),
            _ => false,
        };
        if !fits {
            mismatches.push(Mismatch::SlotBoundMismatch {
                handle: handle.clone(),
                slot: index,
            });
        }
    }
    if outcome_bands.is_empty() {
        mismatches.push(Mismatch::NoOutcomeBand {
            handle: handle.clone(),
        });
    }
    for (index, band) in outcome_bands.iter().enumerate() {
        if band.weight == 0 {
            mismatches.push(Mismatch::ZeroBandWeight {
                handle: handle.clone(),
                band: index,
            });
        }
        if band.effects.iter().any(|slot| *slot >= effect_slots.len()) {
            mismatches.push(Mismatch::DanglingBandEffect {
                handle: handle.clone(),
                band: index,
            });
        }
        if band.effects.windows(2).any(|pair| pair[0] >= pair[1]) {
            mismatches.push(Mismatch::BandEffectsNotCanonical {
                handle: handle.clone(),
                band: index,
            });
        }
    }
    if effect_slots.is_empty() && !carries_speech {
        mismatches.push(Mismatch::InertAffordance {
            handle: handle.clone(),
        });
    }
}

/// The one entity resolver. `expected` names the kind the referring site
/// requires; a handle or ID that answers with another kind is `WrongKind`.
fn resolve_entity(
    site: Site,
    expected_kind: EntityKind,
    reference: &Ref<EntityId>,
    index: &BTreeMap<DraftHandle, RefKind>,
    entities: &BTreeMap<EntityId, EntityRecord>,
    mismatches: &mut Vec<Mismatch>,
) -> Option<Key<EntityId>> {
    let expected = RefKind::Entity(expected_kind);
    match reference {
        Ref::Draft(named) => match index.get(named) {
            None => {
                mismatches.push(Mismatch::UnresolvedDraft {
                    site,
                    referent: named.clone(),
                    expected,
                });
                None
            }
            Some(kind) if *kind != expected => {
                mismatches.push(Mismatch::WrongKind {
                    site,
                    referent: RefName::Entity(reference.clone()),
                    expected,
                    actual: *kind,
                });
                None
            }
            Some(_) => Some(Key::Draft(named.clone())),
        },
        Ref::Existing(entity_id) => match entities.get(entity_id) {
            None => {
                mismatches.push(Mismatch::UnknownCanonical { site, expected });
                None
            }
            Some(record) if record.kind != expected_kind => {
                mismatches.push(Mismatch::WrongKind {
                    site,
                    referent: RefName::Entity(reference.clone()),
                    expected,
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

fn resolve_dependency_target(
    site: Site,
    target: &DependencyRef,
    index: &BTreeMap<DraftHandle, RefKind>,
    state: &super::WorldState,
    mismatches: &mut Vec<Mismatch>,
) -> Option<TargetKey> {
    match target {
        DependencyRef::Resource(reference) => resolve_entity(
            site,
            EntityKind::Resource,
            reference,
            index,
            &state.entities,
            mismatches,
        )
        .map(TargetKey::Resource),
        DependencyRef::Route(reference) => {
            resolve_route(site, reference, index, &state.edges, mismatches).map(TargetKey::Route)
        }
        DependencyRef::Subject(reference) => {
            resolve_subject(site, reference, index, &state.subjects, mismatches)
                .map(TargetKey::Subject)
        }
    }
}

/// What a holder holds in the candidate map. Absence is zero, at both levels.
fn candidate_held(
    holdings: &BTreeMap<(Key<SubjectId>, Key<EntityId>), Quantity>,
    holder: &Key<SubjectId>,
    resource: &Key<EntityId>,
) -> u64 {
    holdings
        .get(&(holder.clone(), resource.clone()))
        .map_or(0, |quantity| quantity.0)
}

/// Zero removes the slot, so one representation of nothing survives the patch.
fn candidate_set(
    holdings: &mut BTreeMap<(Key<SubjectId>, Key<EntityId>), Quantity>,
    holder: &Key<SubjectId>,
    resource: &Key<EntityId>,
    value: u64,
) {
    let slot = (holder.clone(), resource.clone());
    if value == 0 {
        holdings.remove(&slot);
    } else {
        holdings.insert(slot, Quantity(value));
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

    // The kernel-built Speak entry enters the same index as a world-declared
    // one, before the declaration loop, so a world reusing its handle collides
    // as a duplicate handle rather than through a reserved-name check.
    let speak_handle = DraftHandle::new(KERNEL_SPEAK_HANDLE);
    if admits_human {
        index.insert(speak_handle.clone(), RefKind::Affordance);
    }
    let mut kind_names: BTreeSet<AffordanceKindName> = state
        .affordance_catalog
        .values()
        .map(|entry| entry.kind.clone())
        .collect();
    if admits_human {
        kind_names.insert(kernel_speak_entry().kind);
    }

    for (position, declaration) in patch.declarations.iter().enumerate() {
        let (handle, label, kind) = match declaration {
            Declaration::Subject(subject) => (
                &subject.handle,
                Some(&subject.label),
                RefKind::Subject(Some(subject.kind)),
            ),
            Declaration::Entity(entity) => (
                &entity.handle,
                Some(&entity.label),
                RefKind::Entity(entity.kind),
            ),
            Declaration::Route(route) => (&route.handle, Some(&route.label), ROUTE),
            // A catalog entry's name is its `kind`, checked by the entry
            // validator against the tool-name alphabet rather than against the
            // label rule.
            Declaration::Affordance(affordance) => (&affordance.handle, None, RefKind::Affordance),
        };
        let named = is_canonical_text(&handle.0);
        if !named {
            mismatches.push(Mismatch::EmptyHandle { position });
        }
        if label.is_some_and(|label| !is_canonical_text(label)) {
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
        if let Declaration::Affordance(affordance) = declaration {
            validate_affordance(
                &affordance.handle,
                &affordance.kind,
                &affordance.roles,
                &affordance.preconditions,
                &affordance.effect_slots,
                &affordance.outcome_bands,
                affordance.carries_speech,
                &mut mismatches,
            );
            if !kind_names.insert(affordance.kind.clone()) {
                mismatches.push(Mismatch::DuplicateAffordanceKind {
                    handle: affordance.handle.clone(),
                });
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
    let mut holdings: BTreeMap<(Key<SubjectId>, Key<EntityId>), Quantity> = state
        .holdings
        .iter()
        .flat_map(|(subject_id, held)| {
            held.iter().map(move |(entity_id, quantity)| {
                (
                    (Key::Existing(*subject_id), Key::Existing(*entity_id)),
                    *quantity,
                )
            })
        })
        .collect();
    let seeded_holdings = holdings.clone();
    let mut dependencies: BTreeSet<(Key<SubjectId>, TargetKey)> = state
        .dependencies
        .iter()
        .flat_map(|(subject_id, bound)| {
            bound.iter().map(move |target| {
                (
                    Key::Existing(*subject_id),
                    match target {
                        DependencyTarget::Resource(id) => TargetKey::Resource(Key::Existing(*id)),
                        DependencyTarget::Route(id) => TargetKey::Route(Key::Existing(*id)),
                        DependencyTarget::Subject(id) => TargetKey::Subject(Key::Existing(*id)),
                    },
                )
            })
        })
        .collect();
    let mut deltas: BTreeMap<Key<EntityId>, LedgerDelta> = BTreeMap::new();
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
                    } else if let Some(parent) = resolve_entity(
                        Site::Declaration(entity.handle.clone()),
                        EntityKind::Place,
                        reference,
                        &index,
                        &state.entities,
                        &mut mismatches,
                    ) {
                        containers.insert(Key::Draft(entity.handle.clone()), parent);
                    }
                }
            }
            Declaration::Affordance(_) => {}
            Declaration::Subject(subject) => {
                for reference in &subject.affordances {
                    resolve_affordance(
                        Site::Declaration(subject.handle.clone()),
                        reference,
                        &index,
                        &state.affordance_catalog,
                        &mut mismatches,
                    );
                }
                if let Some(reference) = &subject.position
                    && let Some(place) = resolve_entity(
                        Site::Declaration(subject.handle.clone()),
                        EntityKind::Place,
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
                let from = resolve_entity(
                    Site::Declaration(route.handle.clone()),
                    EntityKind::Place,
                    &route.from,
                    &index,
                    &state.entities,
                    &mut mismatches,
                );
                let to = resolve_entity(
                    Site::Declaration(route.handle.clone()),
                    EntityKind::Place,
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
            ComponentOp::Transfer {
                from,
                to,
                resource,
                qty,
            } => {
                let from_key = resolve_subject(
                    Site::Operation(position),
                    from,
                    &index,
                    &state.subjects,
                    &mut mismatches,
                );
                let to_key = resolve_subject(
                    Site::Operation(position),
                    to,
                    &index,
                    &state.subjects,
                    &mut mismatches,
                );
                let resource_key = resolve_entity(
                    Site::Operation(position),
                    EntityKind::Resource,
                    resource,
                    &index,
                    &state.entities,
                    &mut mismatches,
                );
                if qty.0 == 0 {
                    mismatches.push(Mismatch::ZeroQuantity {
                        operation: position,
                    });
                }
                let (Some(from_key), Some(to_key), Some(resource_key)) =
                    (from_key, to_key, resource_key)
                else {
                    continue;
                };
                if from_key == to_key {
                    mismatches.push(Mismatch::NoOperationEffect {
                        operation: position,
                    });
                    continue;
                }
                if qty.0 == 0 {
                    continue;
                }
                let held = candidate_held(&holdings, &from_key, &resource_key);
                let Some(remaining) = held.checked_sub(qty.0) else {
                    mismatches.push(Mismatch::InsufficientCustody {
                        operation: position,
                    });
                    continue;
                };
                let Some(credited) =
                    candidate_held(&holdings, &to_key, &resource_key).checked_add(qty.0)
                else {
                    mismatches.push(Mismatch::QuantityOverflow {
                        operation: position,
                    });
                    continue;
                };
                candidate_set(&mut holdings, &from_key, &resource_key, remaining);
                candidate_set(&mut holdings, &to_key, &resource_key, credited);
                // A transfer contributes to no ledger term: the equation proves
                // it, because moving a different amount out than in breaks the
                // resource's total.
                deltas.entry(resource_key).or_default();
            }
            ComponentOp::Transform {
                holder,
                from_resource,
                into_resource,
                qty,
            } => {
                let holder_key = resolve_subject(
                    Site::Operation(position),
                    holder,
                    &index,
                    &state.subjects,
                    &mut mismatches,
                );
                let from_key = resolve_entity(
                    Site::Operation(position),
                    EntityKind::Resource,
                    from_resource,
                    &index,
                    &state.entities,
                    &mut mismatches,
                );
                let into_key = resolve_entity(
                    Site::Operation(position),
                    EntityKind::Resource,
                    into_resource,
                    &index,
                    &state.entities,
                    &mut mismatches,
                );
                if qty.0 == 0 {
                    mismatches.push(Mismatch::ZeroQuantity {
                        operation: position,
                    });
                }
                let (Some(holder_key), Some(from_key), Some(into_key)) =
                    (holder_key, from_key, into_key)
                else {
                    continue;
                };
                if from_key == into_key {
                    mismatches.push(Mismatch::NoOperationEffect {
                        operation: position,
                    });
                    continue;
                }
                if qty.0 == 0 {
                    continue;
                }
                let Some(remaining) =
                    candidate_held(&holdings, &holder_key, &from_key).checked_sub(qty.0)
                else {
                    mismatches.push(Mismatch::InsufficientCustody {
                        operation: position,
                    });
                    continue;
                };
                let Some(gained) =
                    candidate_held(&holdings, &holder_key, &into_key).checked_add(qty.0)
                else {
                    mismatches.push(Mismatch::QuantityOverflow {
                        operation: position,
                    });
                    continue;
                };
                candidate_set(&mut holdings, &holder_key, &from_key, remaining);
                candidate_set(&mut holdings, &holder_key, &into_key, gained);
                deltas.entry(from_key).or_default().spent += u128::from(qty.0);
                deltas.entry(into_key).or_default().gained += u128::from(qty.0);
            }
            ComponentOp::Consume {
                holder,
                resource,
                qty,
            } => {
                let holder_key = resolve_subject(
                    Site::Operation(position),
                    holder,
                    &index,
                    &state.subjects,
                    &mut mismatches,
                );
                let resource_key = resolve_entity(
                    Site::Operation(position),
                    EntityKind::Resource,
                    resource,
                    &index,
                    &state.entities,
                    &mut mismatches,
                );
                if qty.0 == 0 {
                    mismatches.push(Mismatch::ZeroQuantity {
                        operation: position,
                    });
                }
                let (Some(holder_key), Some(resource_key)) = (holder_key, resource_key) else {
                    continue;
                };
                if qty.0 == 0 {
                    continue;
                }
                let Some(remaining) =
                    candidate_held(&holdings, &holder_key, &resource_key).checked_sub(qty.0)
                else {
                    mismatches.push(Mismatch::InsufficientCustody {
                        operation: position,
                    });
                    continue;
                };
                candidate_set(&mut holdings, &holder_key, &resource_key, remaining);
                deltas.entry(resource_key).or_default().consumed += u128::from(qty.0);
            }
            ComponentOp::Admit {
                holder,
                resource,
                qty,
                evidence,
            } => {
                let holder_key = resolve_subject(
                    Site::Operation(position),
                    holder,
                    &index,
                    &state.subjects,
                    &mut mismatches,
                );
                let resource_key = resolve_entity(
                    Site::Operation(position),
                    EntityKind::Resource,
                    resource,
                    &index,
                    &state.entities,
                    &mut mismatches,
                );
                if qty.0 == 0 {
                    mismatches.push(Mismatch::ZeroQuantity {
                        operation: position,
                    });
                }
                let evidenced = is_canonical_text(&evidence.0) && patch.evidence.contains(evidence);
                if !evidenced {
                    mismatches.push(Mismatch::AdmitWithoutEvidence {
                        operation: position,
                    });
                }
                let (Some(holder_key), Some(resource_key)) = (holder_key, resource_key) else {
                    continue;
                };
                if qty.0 == 0 || !evidenced {
                    continue;
                }
                let Some(admitted) =
                    candidate_held(&holdings, &holder_key, &resource_key).checked_add(qty.0)
                else {
                    mismatches.push(Mismatch::QuantityOverflow {
                        operation: position,
                    });
                    continue;
                };
                candidate_set(&mut holdings, &holder_key, &resource_key, admitted);
                deltas.entry(resource_key).or_default().admitted += u128::from(qty.0);
            }
            ComponentOp::Bind { subject, target } | ComponentOp::Release { subject, target } => {
                let bind = matches!(operation, ComponentOp::Bind { .. });
                let subject_key = resolve_subject(
                    Site::Operation(position),
                    subject,
                    &index,
                    &state.subjects,
                    &mut mismatches,
                );
                let target_key = resolve_dependency_target(
                    Site::Operation(position),
                    target,
                    &index,
                    state,
                    &mut mismatches,
                );
                let (Some(subject_key), Some(target_key)) = (subject_key, target_key) else {
                    continue;
                };
                if target_key == TargetKey::Subject(subject_key.clone()) {
                    mismatches.push(Mismatch::SelfDependency {
                        operation: position,
                    });
                    continue;
                }
                let slot = (subject_key, target_key);
                if dependencies.contains(&slot) == bind {
                    mismatches.push(Mismatch::NoOperationEffect {
                        operation: position,
                    });
                } else if bind {
                    dependencies.insert(slot);
                } else {
                    dependencies.remove(&slot);
                }
            }
        }
    }

    // Conservation, over the complete candidate after every operation has been
    // applied in order. Never per operation: a patch whose operations
    // individually balance but whose net does not is impossible, and an
    // intermediate negative was already caught above as `InsufficientCustody`.
    for (resource_key, delta) in &mut deltas {
        delta.before = seeded_holdings
            .iter()
            .filter(|((_, resource), _)| resource == resource_key)
            .map(|(_, quantity)| u128::from(quantity.0))
            .sum();
        delta.after = holdings
            .iter()
            .filter(|((_, resource), _)| resource == resource_key)
            .map(|(_, quantity)| u128::from(quantity.0))
            .sum();
    }
    if let Some(resource) = check_ledger(&deltas) {
        mismatches.push(Mismatch::CustodyNotConserved {
            resource: RefName::Entity(match resource {
                Key::Existing(entity_id) => Ref::Existing(entity_id),
                Key::Draft(handle) => Ref::Draft(handle),
            }),
        });
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
    let entity_id_of = |key: &Key<EntityId>| -> EntityId {
        match key {
            Key::Existing(entity_id) => *entity_id,
            Key::Draft(handle) => *allocated_entities
                .get(handle)
                .expect("a draft entity reference resolved above"),
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
            .map(|reference| entity_id_of(&key_of(reference)));
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
                from: entity_id_of(&key_of(&route.from)),
                to: entity_id_of(&key_of(&route.to)),
                access: route.access,
                cost: route.cost,
                open: true,
            },
        });
    }

    let mut allocated_affordances: BTreeMap<DraftHandle, AffordanceId> = BTreeMap::new();
    let mut affordances = Vec::new();
    let allocate_affordance = |handle: &DraftHandle,
                               entry: Affordance,
                               affordances: &mut Vec<ResolvedAffordance>,
                               allocated: &mut BTreeMap<DraftHandle, AffordanceId>,
                               collisions: &mut Vec<Mismatch>| {
        let affordance_id = AffordanceId(derive_id(
            AFFORDANCE_NAMESPACE,
            world_id,
            command_id,
            handle,
            None,
        ));
        if state.affordance_catalog.contains_key(&affordance_id) {
            collisions.push(Mismatch::CanonicalCollision {
                handle: handle.clone(),
            });
        }
        allocated.insert(handle.clone(), affordance_id);
        affordances.push(ResolvedAffordance {
            handle: handle.clone(),
            affordance_id,
            affordance: entry,
        });
    };
    if admits_human {
        allocate_affordance(
            &speak_handle,
            kernel_speak_entry(),
            &mut affordances,
            &mut allocated_affordances,
            &mut collisions,
        );
    }
    for declaration in &patch.declarations {
        let Declaration::Affordance(input) = declaration else {
            continue;
        };
        allocate_affordance(
            &input.handle,
            Affordance {
                kind: input.kind.clone(),
                roles: input.roles.clone(),
                preconditions: input.preconditions.clone(),
                effect_slots: input.effect_slots.clone(),
                outcome_bands: input.outcome_bands.clone(),
                carries_speech: input.carries_speech,
            },
            &mut affordances,
            &mut allocated_affordances,
            &mut collisions,
        );
    }
    let affordance_id_of = |key: &Key<AffordanceId>| -> AffordanceId {
        match key {
            Key::Existing(affordance_id) => *affordance_id,
            Key::Draft(handle) => *allocated_affordances
                .get(handle)
                .expect("a draft affordance reference resolved above"),
        }
    };

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
        let granted = input
            .affordances
            .iter()
            .map(|reference| affordance_id_of(&key_of(reference)))
            .collect();
        subjects.push(ResolvedSubject {
            handle: input.handle.clone(),
            subject_id,
            subject: SubjectState {
                label: input.label.clone(),
                kind: input.kind,
            },
            controller,
            affordances: granted,
            position: input.position.as_ref().map(|reference| Position {
                place: entity_id_of(&key_of(reference)),
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
    let target_of = |target: &DependencyRef| -> DependencyTarget {
        match target {
            DependencyRef::Resource(reference) => {
                DependencyTarget::Resource(entity_id_of(&key_of(reference)))
            }
            DependencyRef::Route(reference) => {
                DependencyTarget::Route(edge_id_of(&key_of(reference)))
            }
            DependencyRef::Subject(reference) => {
                DependencyTarget::Subject(subject_id_of(&key_of(reference)))
            }
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
            ComponentOp::Transfer {
                from,
                to,
                resource,
                qty,
            } => ResolvedOp::Transfer {
                from: subject_id_of(&key_of(from)),
                to: subject_id_of(&key_of(to)),
                resource: entity_id_of(&key_of(resource)),
                qty: *qty,
            },
            ComponentOp::Transform {
                holder,
                from_resource,
                into_resource,
                qty,
            } => ResolvedOp::Transform {
                holder: subject_id_of(&key_of(holder)),
                from_resource: entity_id_of(&key_of(from_resource)),
                into_resource: entity_id_of(&key_of(into_resource)),
                qty: *qty,
            },
            ComponentOp::Consume {
                holder,
                resource,
                qty,
            } => ResolvedOp::Consume {
                holder: subject_id_of(&key_of(holder)),
                resource: entity_id_of(&key_of(resource)),
                qty: *qty,
            },
            ComponentOp::Admit {
                holder,
                resource,
                qty,
                evidence,
            } => ResolvedOp::Admit {
                holder: subject_id_of(&key_of(holder)),
                resource: entity_id_of(&key_of(resource)),
                qty: *qty,
                evidence: evidence.clone(),
            },
            ComponentOp::Bind { subject, target } => ResolvedOp::Bind {
                subject: subject_id_of(&key_of(subject)),
                target: target_of(target),
            },
            ComponentOp::Release { subject, target } => ResolvedOp::Release {
                subject: subject_id_of(&key_of(subject)),
                target: target_of(target),
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
        affordances,
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
        activate, admit_topology, auth_principal, command, creation, owner, player, reject_owner,
        speak_entry, submit_owner,
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

    /// Every fixture institution is declared by a post-genesis `AdmitPatch`, so
    /// it grants the committed Speak entry by canonical reference: the kernel
    /// synthesizes that entry once, at genesis, and a later patch names it like
    /// any other structure a previous commit allocated.
    fn institution(
        kernel: &WorldKernel,
        handle: &str,
        label: &str,
        position: Option<Ref<EntityId>>,
    ) -> Declaration {
        Declaration::Subject(SubjectDeclaration {
            handle: DraftHandle::new(handle),
            label: label.into(),
            kind: SubjectKind::Institution,
            controller: NewController::OperationalAgent,
            affordances: BTreeSet::from([speak_entry(kernel)]),
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

    /// A minimal admissible entry, so a validation test changes exactly one
    /// thing and names exactly one rejection.
    fn catalog_entry(
        handle: &str,
        kind: &str,
        roles: Vec<RoleSpec>,
        effect_slots: Vec<EffectSlot>,
        outcome_bands: Vec<OutcomeBand>,
    ) -> Declaration {
        Declaration::Affordance(AffordanceDeclaration {
            handle: DraftHandle::new(handle),
            kind: AffordanceKindName(kind.into()),
            roles,
            preconditions: Vec::new(),
            effect_slots,
            outcome_bands,
            carries_speech: false,
        })
    }

    fn role_spec(role: &str, kind: RefKind) -> RoleSpec {
        RoleSpec {
            role: Role(role.into()),
            kind,
        }
    }

    fn transfer_slot(roles: Vec<&str>, bounds: Bounds) -> EffectSlot {
        EffectSlot {
            op_kind: ComponentOpKind::Transfer,
            roles: roles.into_iter().map(|role| Role(role.into())).collect(),
            bounds,
        }
    }

    fn transfer_roles() -> Vec<RoleSpec> {
        vec![
            role_spec("from", RefKind::Subject(None)),
            role_spec("to", RefKind::Subject(None)),
            role_spec("goods", RefKind::Entity(EntityKind::Resource)),
        ]
    }

    fn band(effects: Vec<usize>) -> OutcomeBand {
        OutcomeBand { weight: 1, effects }
    }

    #[test]
    fn a_catalog_entry_with_a_dangling_or_noncanonical_band_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let before = kernel.snapshot().unwrap();
        let slots = vec![
            transfer_slot(vec!["from", "to", "goods"], Bounds::Quantity(Quantity(2))),
            transfer_slot(vec!["from", "to", "goods"], Bounds::Quantity(Quantity(2))),
        ];
        let handle = DraftHandle::new("verb");

        for (bands, expected) in [
            (
                vec![band(vec![2])],
                Mismatch::DanglingBandEffect {
                    handle: handle.clone(),
                    band: 0,
                },
            ),
            (
                vec![band(vec![1, 0])],
                Mismatch::BandEffectsNotCanonical {
                    handle: handle.clone(),
                    band: 0,
                },
            ),
            (
                vec![OutcomeBand {
                    weight: 0,
                    effects: vec![0],
                }],
                Mismatch::ZeroBandWeight {
                    handle: handle.clone(),
                    band: 0,
                },
            ),
            (
                Vec::new(),
                Mismatch::NoOutcomeBand {
                    handle: handle.clone(),
                },
            ),
        ] {
            let rejected = reject_owner(
                &mut kernel,
                &before,
                admit(patch_of(vec![catalog_entry(
                    "verb",
                    "verb",
                    transfer_roles(),
                    slots.clone(),
                    bands,
                )])),
            );
            assert_eq!(rejected, vec![expected]);
            assert!(kernel.state.affordance_catalog.len() == before.affordances.len());
        }

        // No effect slots and no speech: the entry could never change anything.
        let rejected = reject_owner(
            &mut kernel,
            &before,
            admit(patch_of(vec![catalog_entry(
                "verb",
                "verb",
                Vec::new(),
                Vec::new(),
                vec![band(Vec::new())],
            )])),
        );
        assert_eq!(rejected, vec![Mismatch::InertAffordance { handle }]);
    }

    #[test]
    fn a_slot_whose_roles_do_not_match_its_operation_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let before = kernel.snapshot().unwrap();
        let handle = DraftHandle::new("verb");

        // A `Transfer` takes three referents, not two.
        assert_eq!(
            reject_owner(
                &mut kernel,
                &before,
                admit(patch_of(vec![catalog_entry(
                    "verb",
                    "verb",
                    transfer_roles(),
                    vec![transfer_slot(
                        vec!["from", "to"],
                        Bounds::Quantity(Quantity(2))
                    )],
                    vec![band(vec![0])],
                )])),
            ),
            vec![Mismatch::SlotRoleArity {
                handle: handle.clone(),
                slot: 0
            }]
        );

        // Its third referent is a resource, not a place.
        assert_eq!(
            reject_owner(
                &mut kernel,
                &before,
                admit(patch_of(vec![catalog_entry(
                    "verb",
                    "verb",
                    vec![
                        role_spec("from", RefKind::Subject(None)),
                        role_spec("to", RefKind::Subject(None)),
                        role_spec("goods", RefKind::Entity(EntityKind::Place)),
                    ],
                    vec![transfer_slot(
                        vec!["from", "to", "goods"],
                        Bounds::Quantity(Quantity(2))
                    )],
                    vec![band(vec![0])],
                )])),
            ),
            vec![Mismatch::RoleKindUnfit {
                handle: handle.clone(),
                role: Role("goods".into())
            }]
        );

        // A `Transfer` carries a quantity, so an unbounded slot has no ceiling.
        assert_eq!(
            reject_owner(
                &mut kernel,
                &before,
                admit(patch_of(vec![catalog_entry(
                    "verb",
                    "verb",
                    transfer_roles(),
                    vec![transfer_slot(vec!["from", "to", "goods"], Bounds::None)],
                    vec![band(vec![0])],
                )])),
            ),
            vec![Mismatch::SlotBoundMismatch {
                handle: handle.clone(),
                slot: 0
            }]
        );

        // A slot naming a role the entry does not declare.
        assert_eq!(
            reject_owner(
                &mut kernel,
                &before,
                admit(patch_of(vec![catalog_entry(
                    "verb",
                    "verb",
                    transfer_roles(),
                    vec![transfer_slot(
                        vec!["from", "to", "cargo"],
                        Bounds::Quantity(Quantity(2))
                    )],
                    vec![band(vec![0])],
                )])),
            ),
            vec![Mismatch::UnknownRole {
                handle,
                role: Role("cargo".into())
            }]
        );
    }

    #[test]
    fn a_grant_naming_an_undeclared_affordance_is_rejected_and_the_same_patch_commits() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let before = kernel.snapshot().unwrap();
        let entries_before = kernel.state.affordance_catalog.len();
        let subjects_before = kernel.state.subjects.len();

        let granting = |handle: &str| {
            Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new("late-arrival"),
                label: "A Late Arrival".into(),
                kind: SubjectKind::Person,
                controller: NewController::NarrativePersona,
                affordances: BTreeSet::from([Ref::Draft(DraftHandle::new(handle))]),
                position: None,
            })
        };

        assert_eq!(
            reject_owner(
                &mut kernel,
                &before,
                admit(patch_of(vec![granting("verb")]))
            ),
            vec![Mismatch::UnresolvedDraft {
                site: Site::Declaration(DraftHandle::new("late-arrival")),
                referent: DraftHandle::new("verb"),
                expected: RefKind::Affordance,
            }]
        );
        assert_eq!(kernel.state.affordance_catalog.len(), entries_before);
        assert_eq!(kernel.state.subjects.len(), subjects_before);

        // Declaring the entry in the same patch commits both atomically.
        let receipt = submit_owner(
            &mut kernel,
            &before,
            admit(patch_of(vec![
                catalog_entry(
                    "verb",
                    "verb",
                    transfer_roles(),
                    vec![transfer_slot(
                        vec!["from", "to", "goods"],
                        Bounds::Quantity(Quantity(2)),
                    )],
                    vec![band(vec![0])],
                ),
                granting("verb"),
            ])),
        );
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
        assert_eq!(kernel.state.affordance_catalog.len(), entries_before + 1);
        assert_eq!(kernel.state.subjects.len(), subjects_before + 1);
        let granted = kernel
            .state
            .affordance_grants
            .values()
            .find(|entries| entries.len() == 1)
            .expect("the new subject's grant set");
        assert!(
            granted
                .iter()
                .all(|id| kernel.state.affordance_catalog.contains_key(id))
        );
    }

    #[test]
    fn two_entries_cannot_share_one_kind_name() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let before = kernel.snapshot().unwrap();
        let slot = transfer_slot(vec!["from", "to", "goods"], Bounds::Quantity(Quantity(2)));
        let rejected = reject_owner(
            &mut kernel,
            &before,
            admit(patch_of(vec![
                catalog_entry(
                    "first",
                    "haul",
                    transfer_roles(),
                    vec![slot.clone()],
                    vec![band(vec![0])],
                ),
                catalog_entry(
                    "second",
                    "haul",
                    transfer_roles(),
                    vec![slot],
                    vec![band(vec![0])],
                ),
            ])),
        );
        assert_eq!(
            rejected,
            vec![Mismatch::DuplicateAffordanceKind {
                handle: DraftHandle::new("second")
            }]
        );

        // A kind name outside the tool-name alphabet is refused for the same
        // reason: it becomes a generated tool name.
        let rejected = reject_owner(
            &mut kernel,
            &before,
            admit(patch_of(vec![catalog_entry(
                "loud",
                "Haul It",
                transfer_roles(),
                vec![transfer_slot(
                    vec!["from", "to", "goods"],
                    Bounds::Quantity(Quantity(2)),
                )],
                vec![band(vec![0])],
            )])),
        );
        assert_eq!(
            rejected,
            vec![Mismatch::InvalidAffordanceName {
                handle: DraftHandle::new("loud")
            }]
        );
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
                        &kernel,
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

        let patch = patch_of(vec![
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
                &kernel,
                "rhythm-authority",
                "The Rhythm Authority",
                Some(Ref::Draft(draft("kharad-rhythm-road"))),
            ),
        ]);
        let receipt = submit_owner(&mut kernel, &before, admit(patch));
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
                            &kernel,
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
                &kernel,
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
                affordances: BTreeSet::from([speak_entry(&kernel)]),
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
                &kernel,
                "rhythm-authority",
                "The Rhythm Authority",
                Some(Ref::Draft(DraftHandle::new("cavity-yard"))),
            ),
            Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new("late-arrival"),
                label: "A Late Arrival".into(),
                kind: SubjectKind::Person,
                controller: NewController::NarrativePersona,
                affordances: BTreeSet::from([speak_entry(&kernel)]),
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
                    &kernel,
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
            affordances: BTreeSet::from([speak_entry(&kernel)]),
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
                        &kernel,
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
                &kernel,
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
            CommandId::issue(),
            &CallerId::Principal(owner()),
            &super::super::WorldEffect::PatchAdmitted { resolved: forged },
        )
        .unwrap_err();
        assert!(matches!(forged_error, KernelError::Invariant(_)));

        // Stale: the honest effect applied twice collides on its own IDs.
        let mut candidate = kernel.state.clone();
        super::super::apply_effect(
            &mut candidate,
            CommandId::issue(),
            &CallerId::Principal(owner()),
            &honest,
        )
        .unwrap();
        let stale_error = super::super::apply_effect(
            &mut candidate,
            CommandId::issue(),
            &CallerId::Principal(owner()),
            &honest,
        )
        .unwrap_err();
        assert!(matches!(stale_error, KernelError::Invariant(_)));

        // A non-owner caller never reaches admission.
        let mut candidate = kernel.state.clone();
        let unauthorized = super::super::apply_effect(
            &mut candidate,
            CommandId::issue(),
            &CallerId::Principal(player()),
            &honest,
        )
        .unwrap_err();
        assert!(matches!(unauthorized, KernelError::Invariant(_)));
        assert_eq!(candidate, kernel.state);

        // And the Active phase is refused before any partition is touched.
        let active = activate(&mut kernel);
        assert_eq!(active.revision, before.revision + 3);
        let mut candidate = kernel.state.clone();
        let wrong_phase = super::super::apply_effect(
            &mut candidate,
            CommandId::issue(),
            &CallerId::Principal(owner()),
            &honest,
        )
        .unwrap_err();
        assert!(matches!(wrong_phase, KernelError::Invariant(_)));
        assert_eq!(candidate, kernel.state);
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
                    affordances: Vec::new(),
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
                CommandId::issue(),
                &CallerId::Principal(owner()),
                &forged,
            )
            .unwrap_err();
            assert!(matches!(error, KernelError::Invariant(_)));
            assert_eq!(candidate, kernel.state);
        }
    }
}
