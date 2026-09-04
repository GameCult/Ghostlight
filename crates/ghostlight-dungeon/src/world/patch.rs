//! Closed-patch resolution: the one owner of draft handles, reference kinds, and
//! declaration-lane ID allocation.
//!
//! Only [`derive_id`] allocates a canonical ID, and it is called from exactly
//! two sites: [`resolve_patch`] here, and `action::exercise`, which mints one
//! referent per speech-carrying invocation — always a `Claimed` fact asserted by
//! the acting subject.
//!
//! A patch names structure two ways and no other way: an exact canonical ID that
//! already keys a partition, or a draft handle declared in the same patch.
//! Resolution accumulates the complete mismatch set first and allocates only
//! after that set is empty, so a rejected patch never mints an ID.

use super::clock::{FictionalMinutes, TickMinutes};
use super::tool_schema;
use super::{
    AffordanceId, CommandId, ControllerAssignment, ControllerId, EdgeId, EntityId, NewController,
    SubjectId, SubjectKind, SubjectState, WorldId,
};
use codex_connector::CodexToolDefinition;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
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

/// Traversal rule. A `Restricted` route names the authority kind that opens it:
/// the door names its own key, and the kernel reads neither the door nor the key
/// beyond comparing the name. [`route_admits`] is the sole statement of who gets
/// through, reached identically by `resolve_patch`'s `Relocate` arm and by
/// `Precondition::Reachable`'s edge admission.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "access", rename_all = "snake_case")]
pub(crate) enum AccessKind {
    Public,
    Restricted { requires: AuthorityKindName },
}

/// A world-declared authority name: canonical text, `[a-z][a-z0-9_]{0,47}`. The
/// kernel carries it, compares it for equality, and reads it no other way. A
/// closed enum here would make "conscript" or "audit" a kernel change and put a
/// world's political vocabulary in the reducer.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct AuthorityKindName(pub(crate) String);

/// What an authority covers. Distinct from `DecisionScope`, which says *whose*
/// authority it is and stays one field.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "over", content = "id", rename_all = "snake_case")]
pub(crate) enum AuthorityTarget {
    Subject(SubjectId),
    /// That place and everything contained in it, by `EntityRecord.container`.
    PlaceSubtree(EntityId),
}

impl AuthorityTarget {
    /// The referent this target names, so "does grant A cover grant B's ground"
    /// is the one covering predicate rather than a second one.
    pub(super) fn as_referent(self) -> super::Target {
        match self {
            Self::Subject(subject_id) => super::Target::Subject(subject_id),
            Self::PlaceSubtree(entity_id) => super::Target::Entity(entity_id),
        }
    }
}

/// The proposal-time twin of [`AuthorityTarget`], carrying references.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "over", content = "ref", rename_all = "snake_case")]
pub(crate) enum AuthorityTargetRef {
    Subject(Ref<SubjectId>),
    PlaceSubtree(Ref<EntityId>),
}

/// One jurisdiction: a kind and the ground it runs over. Ordered `(kind, over)`
/// so a subject's grant set has one canonical order. A jurisdiction is a set of
/// these, so revoking one place is not a restatement of the whole scope.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuthorityGrant {
    pub(crate) kind: AuthorityKindName,
    pub(crate) over: AuthorityTarget,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuthorityGrantRef {
    pub(crate) kind: AuthorityKindName,
    pub(crate) over: AuthorityTargetRef,
}

/// Canonical text, scoped to its institution: the pair (institution, name) is
/// the identity, the same discipline a `Role` gets inside one affordance.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct OfficeName(pub(crate) String);

/// A seat inside one institution. It carries no term and no selection method:
/// there is no clock, so nothing can expire, and a field nothing reads is a
/// decoration the next pass has to delete. `delegated` is the field that earns
/// its keep — `scope_components` reads it, `Authorized` resolves through it, and
/// the scope digest binds it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Office {
    /// A person subject, or vacant. A vacancy is an ordinary state.
    pub(crate) incumbent: Option<SubjectId>,
    /// Which of the institution's authority kinds this office lends its
    /// incumbent. Non-empty: an office lending nothing is inert.
    pub(crate) delegated: BTreeSet<AuthorityKindName>,
}

/// World-declared canonical text, kernel-opaque, `[a-z][a-z0-9_]{0,47}`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct GrievanceKindName(pub(crate) String);

/// Where one kind of grievance goes, and who may bring it. One forum per
/// grievance kind, world-wide; per-jurisdiction forums would key this on
/// `(GrievanceKindName, AuthorityTarget)`, and that key has no consumer yet.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Forum {
    /// Any subject. A one-person magistrate is a legitimate forum, so the
    /// kernel branches on `SubjectKind` here as it does everywhere else: not at
    /// all.
    pub(crate) forum: SubjectId,
    /// Who may bring this grievance. It reuses `AuthorityTarget`'s type and its
    /// covering predicate and is emphatically not stored in the `authority`
    /// partition, so `RevokeAuthority` cannot strip standing.
    pub(crate) standing: AuthorityTarget,
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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "namespace", content = "ref", rename_all = "snake_case")]
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

/// Committed world text: canonical, non-empty. The kernel stores it, hands it to
/// a projection, and reads it no other way. Nothing compares two `Statement`s:
/// no dedup, no interning, no contradiction check, no similarity. Two facts with
/// byte-identical statements are two facts.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct Statement(String);

impl Statement {
    pub(crate) fn new(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        is_canonical_text(&value).then_some(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// How a fact stands. Immutable after admission: there is no `SetStanding`. A
/// claim never becomes canon and canon never degrades to a claim, so nothing
/// ever has to decide which of two facts wins. A world in which a court rules on
/// a rumour declares a *new* `Canonical` fact; the rumour survives beside it,
/// which is what makes a later `Redress` reading possible at all.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "standing", rename_all = "snake_case")]
pub(crate) enum FactStanding {
    /// Admitted with a receipt, through the same evidence gate `Admit` uses.
    Canonical { evidence: EvidenceRef },
    /// Asserted by a subject. The kernel does not evaluate the assertion.
    Claimed { by: SubjectId },
}

/// The proposal-time twin of [`FactStanding`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "standing", rename_all = "snake_case")]
pub(crate) enum FactStandingRef {
    Canonical { evidence: EvidenceRef },
    Claimed { by: Ref<SubjectId> },
}

/// One `EntityKind::Fact` row's payload. Write-once: only declaration and
/// `AssertClaim` ever write `facts`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct FactRecord {
    pub(crate) statement: Statement,
    pub(crate) standing: FactStanding,
}

/// Where a channel carries. A place reach is the subtree, through the same
/// [`covers_place`] walk a `PlaceSubtree` jurisdiction uses.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "reach", content = "of", rename_all = "snake_case")]
pub(crate) enum Reach {
    /// Exactly these subjects. May be empty: an empty reach is the named
    /// silenced state, which is the outcome this component exists to hold.
    Subjects(BTreeSet<SubjectId>),
    /// Everyone positioned at that place or anywhere inside it.
    Place(EntityId),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "reach", content = "of", rename_all = "snake_case")]
pub(crate) enum ReachRef {
    Subjects(BTreeSet<Ref<SubjectId>>),
    Place(Ref<EntityId>),
}

/// One `EntityKind::Channel` row's payload. It carries no latency: there is no
/// clock, so a latency field would be read by nothing and bound by nothing.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChannelRecord {
    pub(crate) reach: Reach,
    /// Who may speak on it besides those inside its reach, and whose scope
    /// digest binds it. The horn belongs to the temple, not to whoever happens
    /// to be within earshot of it.
    pub(crate) controller: Option<SubjectId>,
}

/// A small closed ordinal. Ordered ascending by declaration order, so derived
/// `Ord` *is* the semantics and `Knows { at_least }` is one `>=`. Three levels:
/// two cannot express "I heard it but I doubt it", which is the state deception
/// produces; four or more is a scale nothing in the vocabulary distinguishes.
/// There is no zero — forgetting is a removed key, exactly as `Quantity(0)` is
/// unstorable.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Confidence {
    Doubted,
    Believed,
    Certain,
}

/// Three sources, three writers, no overlap: `Witnessed` and `Evidenced` are
/// written by `AcquireKnowledge`, `Told` only by `Communicate`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "source", rename_all = "snake_case")]
pub(crate) enum KnowledgeSource {
    Witnessed,
    /// `via: None` is co-location.
    Told {
        by: SubjectId,
        via: Option<EntityId>,
    },
    Evidenced,
}

/// What one subject holds of one fact.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Knowledge {
    pub(crate) confidence: Confidence,
    pub(crate) source: KnowledgeSource,
}

/// The source an author may write. `Told` is unrepresentable here: only
/// `Communicate` writes it, so no author can forge a teller.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "source", rename_all = "snake_case")]
pub(crate) enum AuthoredSource {
    Witnessed,
    Evidenced,
}

/// Where a telling lands. `Colocated` is derived from `positions` at check and
/// at apply; it is never a stored channel, so no entity is minted per place and
/// no world must enumerate a village to let people talk in a room.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Audience {
    Colocated,
    Channel(EntityId),
}

impl Audience {
    pub(super) fn channel(self) -> Option<EntityId> {
        match self {
            Self::Colocated => None,
            Self::Channel(entity_id) => Some(entity_id),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AudienceRef {
    Colocated,
    Channel(Ref<EntityId>),
}

/// The catalog form: a declared affordance names a role, the invoker binds a
/// channel to it, and `exercise` lowers this plus the bindings into the
/// canonical [`Audience`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AudienceSpec {
    Colocated,
    Channel(Role),
}

/// What a subject has promised. The kernel branches on this twice — the tick
/// behaves differently per kind and the scale count reads `Goal` — so it is a
/// closed enum rather than a world-declared name.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CommitmentKind {
    Routine,
    Obligation,
    Goal,
}

/// Deterministic from the creating command and the operation's index within
/// that command's lowered operations. No `EdgeId`: nothing references a
/// commitment by identity, so a structural key is the whole answer.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommitmentKey {
    pub(crate) command: CommandId,
    pub(crate) index: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Commitment {
    pub(crate) kind: CommitmentKind,
    /// `None` is a promise to oneself: a personal `Goal`. A counterparty equal
    /// to the subject is refused.
    pub(crate) counterparty: Option<SubjectId>,
    /// Absolute. "Past due" is `due <= now`: one comparison, no countdown
    /// fanned across the partition.
    pub(crate) due: FictionalMinutes,
    /// Required for `Routine` — that is what makes it recur — and forbidden
    /// otherwise. On auto-fulfilment `due` rolls forward by this.
    pub(crate) period: Option<TickMinutes>,
    /// What must hold for a `Routine` to auto-fulfil. Role-free canonical
    /// checks. Empty for `Obligation` and `Goal`.
    pub(crate) checks: Vec<BoundPrecondition>,
}

/// Nonzero by construction; saturating in both directions. A separate newtype
/// from [`super::Magnitude`], which is an effect ceiling and shares nothing with
/// this but a word.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct PressureMagnitude(pub(crate) u32);

/// Target-major, because every reader is: the attention order reads pressure
/// *on* a subject and the typed view shows pressure on self. Source-major has no
/// reader.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "from", content = "of", rename_all = "snake_case")]
pub(crate) enum PressureSource {
    /// A past-due commitment. Carries the promisor and the structural key.
    Commitment {
        subject: SubjectId,
        key: CommitmentKey,
    },
    /// An unavailable dependency of the pressed subject. This is how a closed
    /// route in one realm becomes a political problem in another without anyone
    /// deciding that it should be.
    Dependency(DependencyTarget),
    /// Another subject, pressing directly through an affordance effect.
    Subject(SubjectId),
}

/// The proposal-time twin of [`PressureSource`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "from", content = "of", rename_all = "snake_case")]
pub(crate) enum PressureSourceRef {
    Commitment {
        subject: Ref<SubjectId>,
        key: CommitmentKey,
    },
    Dependency(DependencyRef),
    Subject(Ref<SubjectId>),
}

/// The role-free canonical twin of [`Precondition`]. A `Precondition` names
/// roles; a `BoundPrecondition` names referents, so a commitment's checks are
/// storable with no second binding path and one evaluator serves both the
/// invocation pipeline and the tick.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "precondition", rename_all = "snake_case")]
pub(crate) enum BoundPrecondition {
    Present {
        at: EntityId,
    },
    Reachable {
        to: EntityId,
        within: Cost,
    },
    Holds {
        resource: EntityId,
        at_least: Quantity,
    },
    Authorized {
        over: super::Target,
        kind: AuthorityKindName,
    },
    HasStanding {
        grievance: GrievanceKindName,
    },
    Knows {
        fact: EntityId,
        at_least: Confidence,
    },
    CanBroadcast {
        via: Audience,
    },
    CanReach {
        subject: SubjectId,
        via: Audience,
    },
    Committed {
        to: SubjectId,
        kind: CommitmentKind,
    },
}

/// The proposal-time twin of [`BoundPrecondition`], carried by
/// `CreateCommitment` so a patch may name a check over structure it declares.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "precondition", rename_all = "snake_case")]
pub(crate) enum PreconditionRef {
    Present {
        at: Ref<EntityId>,
    },
    Reachable {
        to: Ref<EntityId>,
        within: Cost,
    },
    Holds {
        resource: Ref<EntityId>,
        at_least: Quantity,
    },
    Authorized {
        over: AuthorityTargetRef,
        kind: AuthorityKindName,
    },
    HasStanding {
        grievance: GrievanceKindName,
    },
    Knows {
        fact: Ref<EntityId>,
        at_least: Confidence,
    },
    CanBroadcast {
        via: AudienceRef,
    },
    CanReach {
        subject: Ref<SubjectId>,
        via: AudienceRef,
    },
    Committed {
        to: Ref<SubjectId>,
        kind: CommitmentKind,
    },
}

/// The authored scale target: how many qualified subjects of each kind the
/// world means to hold, and how that target distributes over jurisdiction
/// roots. Written once by genesis and never mutated. Not a component of any
/// referent, so not a `Declaration` and not a partition.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorldScaleIntent {
    /// Target qualified subjects per kind, world-wide.
    pub(crate) targets: BTreeMap<SubjectKind, u32>,
    /// Jurisdiction roots and their share, in permille. Weights distribute the
    /// target and never raise it: the sum is checked `<= 1000`.
    pub(crate) jurisdictions: BTreeMap<EntityId, u32>,
}

/// The genesis-lane twin: the intent names places the same patch declares, and
/// there is no other way to name them before they exist.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorldScaleIntentRef {
    pub(crate) targets: BTreeMap<SubjectKind, u32>,
    pub(crate) jurisdictions: BTreeMap<DraftHandle, u32>,
}

/// One counted region of the scale deficit. `Uncovered` is the residual: a
/// subject with no jurisdiction root covering it — standing nowhere, or
/// standing somewhere outside every declared root's subtree — is counted and
/// visible while reducing no target.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "jurisdiction", content = "root", rename_all = "snake_case")]
pub(crate) enum JurisdictionKey {
    PlaceSubtree(EntityId),
    Uncovered,
}

/// What must already be true of committed state before an invocation is
/// admitted.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "precondition", rename_all = "snake_case")]
pub(crate) enum Precondition {
    /// The acting subject's Position names the place bound to `at`.
    Present { at: Role },
    /// A path exists from the actor's place to the place bound to `to`, over
    /// open routes this subject may traverse, with summed cost at most `within`.
    Reachable { to: Role, within: Cost },
    /// The acting subject's own holding of the resource bound to `resource` is
    /// at least `at_least`.
    Holds { resource: Role, at_least: Quantity },
    /// The acting subject's effective authority — its own grants plus what
    /// every office it occupies lends — holds a grant of `kind` whose target
    /// covers the referent bound to `over`.
    Authorized { over: Role, kind: AuthorityKindName },
    /// A forum takes `grievance` and the acting subject is inside its standing.
    HasStanding { grievance: GrievanceKindName },
    /// The acting subject holds the fact bound to `fact` at `at_least` or
    /// better.
    Knows { fact: Role, at_least: Confidence },
    /// The acting subject has an audience at all: it is positioned, for
    /// co-location, or it is inside the bound channel's reach or is its
    /// controller.
    CanBroadcast { via: AudienceSpec },
    /// The referent bound to `subject` is inside that audience. Addressing does
    /// not narrow the audience: a telling still lands on everyone in it.
    CanReach { subject: Role, via: AudienceSpec },
    /// The acting subject holds a commitment of `kind` to the referent bound to
    /// `to`. This is what lets a world author an affordance whose legitimacy is
    /// a promise rather than a jurisdiction.
    Committed { to: Role, kind: CommitmentKind },
}

/// Exactly the operations an affordance may propose. `Admit` is absent because
/// minting quantity requires an `EvidenceRef` and an invocation carries no
/// evidence list, so admitting it here would be a second creation path beside
/// the single evidenced one. `OpenOffice`, `CloseOffice`, `OpenForum`, and
/// `CloseForum` are absent because constituting an office or a forum has no
/// live in-play reader an affordance would serve; they stay patch-lane only.
///
/// The four civic variants carry their payload on the variant rather than on
/// the slot or the invocation: an authority kind and an office name are not
/// referents, so they cannot be roles, and a proposer's only degree of freedom
/// stays magnitude. The world fixes the kind when it authors the entry.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "op", rename_all = "snake_case")]
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
    GrantAuthority {
        kind: AuthorityKindName,
    },
    RevokeAuthority {
        kind: AuthorityKindName,
    },
    InstallIncumbent {
        office: OfficeName,
    },
    VacateOffice {
        office: OfficeName,
    },
    /// Source is fixed to `Witnessed`: only `Communicate` writes a teller, and
    /// `Evidenced` needs a patch's evidence list, which an invocation has not.
    AcquireKnowledge {
        confidence: Confidence,
    },
    Forget,
    /// The kind, the horizon, and the period are fixed on the variant, so a
    /// proposer's only degree of freedom stays magnitude. `due` is
    /// `now + horizon`, computed by the kernel at lowering: an absolute due on
    /// a catalog entry would be a fixed date the world outgrows.
    CreateCommitment {
        kind: CommitmentKind,
        horizon: TickMinutes,
        period: Option<TickMinutes>,
    },
    /// The source is fixed to the acting subject, so it cannot be forged. A
    /// world authoring `threaten`, `reassure`, or `forgive` needs these three.
    AdvancePressure {
        by: PressureMagnitude,
    },
    ReducePressure {
        by: PressureMagnitude,
    },
    ResolvePressure,
}

/// The referent shape of one operation: the single source of both the
/// declaration check and the lowering.
pub(super) enum RoleKindRule {
    Exact(RefKind),
    AnyDependencyTarget,
    AnyAuthorityTarget,
}

/// Which magnitude, if any, an operation carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BoundsDimension {
    None,
    Quantity,
    Cost,
}

impl ComponentOpKind {
    pub(super) fn arity(&self) -> Vec<RoleKindRule> {
        let subject = || RoleKindRule::Exact(ANY_SUBJECT);
        let route = || RoleKindRule::Exact(ROUTE);
        let resource = || RoleKindRule::Exact(RefKind::Entity(EntityKind::Resource));
        match self {
            Self::Relocate => vec![subject(), route()],
            Self::OpenRoute | Self::CloseRoute | Self::AlterCost => vec![route()],
            Self::Transfer => vec![subject(), subject(), resource()],
            Self::Transform => vec![subject(), resource(), resource()],
            Self::Consume => vec![subject(), resource()],
            Self::Bind | Self::Release => vec![subject(), RoleKindRule::AnyDependencyTarget],
            Self::GrantAuthority { .. } | Self::RevokeAuthority { .. } => {
                vec![subject(), RoleKindRule::AnyAuthorityTarget]
            }
            Self::InstallIncumbent { .. } => vec![subject(), subject()],
            Self::VacateOffice { .. } => vec![subject()],
            Self::AcquireKnowledge { .. } | Self::Forget => {
                vec![subject(), RoleKindRule::Exact(FACT)]
            }
            // Promisor and counterparty. A `Goal` slot is refused at admission
            // with `GoalWithCounterparty`: a promise to oneself has no second
            // referent to bind, so the action lane authors `Routine` and
            // `Obligation` and the patch lane authors goals.
            Self::CreateCommitment { .. } => vec![subject(), subject()],
            Self::AdvancePressure { .. } | Self::ReducePressure { .. } | Self::ResolvePressure => {
                vec![subject()]
            }
        }
    }

    pub(super) fn dimension(&self) -> BoundsDimension {
        match self {
            Self::Transfer | Self::Transform | Self::Consume => BoundsDimension::Quantity,
            Self::AlterCost => BoundsDimension::Cost,
            _ => BoundsDimension::None,
        }
    }

    /// The civic names this variant carries, so one validator checks them all.
    fn payload_names(&self) -> Vec<&str> {
        match self {
            Self::GrantAuthority { kind } | Self::RevokeAuthority { kind } => vec![&kind.0],
            Self::InstallIncumbent { office } | Self::VacateOffice { office } => vec![&office.0],
            _ => Vec::new(),
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
        RoleKindRule::AnyAuthorityTarget => {
            matches!(declared, RefKind::Subject(_) | PLACE)
        }
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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "site", content = "at", rename_all = "snake_case")]
pub(crate) enum Site {
    Declaration(DraftHandle),
    Operation(usize),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct EvidenceRef(String);

impl EvidenceRef {
    /// The elaborator's evidence receipts are built here, and `Deserialize`
    /// already mints one from any string, so this constructor gates nothing.
    /// Canonical text is checked by the resolver, in the complete mismatch set.
    pub(super) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub(super) fn text(&self) -> &str {
        &self.0
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

/// A fact's label is a short authored name ("the flooding of the lower hinge"),
/// never a transcript: the statement is never copied into the label and the
/// label is never derived from the statement, so the utterance has exactly one
/// home in state.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct FactDeclaration {
    pub(crate) handle: DraftHandle,
    pub(crate) label: String,
    pub(crate) statement: Statement,
    pub(crate) standing: FactStandingRef,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChannelDeclaration {
    pub(crate) handle: DraftHandle,
    pub(crate) label: String,
    pub(crate) reach: ReachRef,
    pub(crate) controller: Option<Ref<SubjectId>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Declaration {
    Subject(SubjectDeclaration),
    /// Places and resources only. `Fact` and `Channel` carry a payload and have
    /// their own declarations, so a referenceable empty one is unrepresentable.
    Entity(EntityDeclaration),
    Route(RouteDeclaration),
    Affordance(AffordanceDeclaration),
    Fact(FactDeclaration),
    Channel(ChannelDeclaration),
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
    /// Adds one grant. An identical grant is `NoOperationEffect`; one that
    /// overlaps another the holder already has is `OverlappingJurisdiction`.
    GrantAuthority {
        holder: Ref<SubjectId>,
        grant: AuthorityGrantRef,
    },
    RevokeAuthority {
        holder: Ref<SubjectId>,
        grant: AuthorityGrantRef,
    },
    /// Creates or reconstitutes an office, preserving any sitting incumbent.
    /// Clipping an office's powers under a sitting incumbent is a political
    /// act, and this is how it is written.
    OpenOffice {
        institution: Ref<SubjectId>,
        office: OfficeName,
        delegated: BTreeSet<AuthorityKindName>,
    },
    CloseOffice {
        institution: Ref<SubjectId>,
        office: OfficeName,
    },
    InstallIncumbent {
        institution: Ref<SubjectId>,
        office: OfficeName,
        incumbent: Ref<SubjectId>,
    },
    VacateOffice {
        institution: Ref<SubjectId>,
        office: OfficeName,
    },
    OpenForum {
        grievance: GrievanceKindName,
        forum: Ref<SubjectId>,
        standing: AuthorityTargetRef,
    },
    CloseForum {
        grievance: GrievanceKindName,
    },
    /// Source is authored: `Witnessed` by an author or an affordance,
    /// `Evidenced` only over a `Canonical` fact.
    AcquireKnowledge {
        subject: Ref<SubjectId>,
        fact: Ref<EntityId>,
        source: AuthoredSource,
        confidence: Confidence,
    },
    /// One telling. It stores the audience, never the recipients: the fan-out is
    /// re-derived at apply from live `positions` and `channels`.
    Communicate {
        speaker: Ref<SubjectId>,
        fact: Ref<EntityId>,
        to: AudienceRef,
    },
    Forget {
        subject: Ref<SubjectId>,
        fact: Ref<EntityId>,
    },
    SetReach {
        channel: Ref<EntityId>,
        reach: ReachRef,
    },
    SetController {
        channel: Ref<EntityId>,
        controller: Option<Ref<SubjectId>>,
    },
    /// Two identical creations are two commitments, not `NoOperationEffect`:
    /// the key is command-derived so they cannot collide, and two promises of
    /// the same thing to the same counterparty are two promises. This is where
    /// a commitment differs from a grant, a holding, and a dependency, all of
    /// which are set-shaped.
    CreateCommitment {
        subject: Ref<SubjectId>,
        counterparty: Option<Ref<SubjectId>>,
        kind: CommitmentKind,
        due: FictionalMinutes,
        period: Option<TickMinutes>,
        checks: Vec<PreconditionRef>,
    },
    /// Removes the commitment and every pressure row sourced by it. Fulfilment,
    /// default, and release are one write: the kernel never learns who invoked
    /// an operation, so three names would be three spellings of one removal
    /// with a distinction only a consumer can read.
    DischargeCommitment {
        subject: Ref<SubjectId>,
        key: CommitmentKey,
    },
    /// Insert-or-add, saturating. With absence meaning zero, creation and
    /// advance are the same write.
    AdvancePressure {
        source: PressureSourceRef,
        target: Ref<SubjectId>,
        by: PressureMagnitude,
    },
    /// Saturating subtract; removal at zero.
    ReducePressure {
        source: PressureSourceRef,
        target: Ref<SubjectId>,
        by: PressureMagnitude,
    },
    ResolvePressure {
        source: PressureSourceRef,
        target: Ref<SubjectId>,
    },
}

/// What an Active `AdmitPatch` answers. Draft answers nothing; Active must
/// answer a boundary the kernel currently derives or a jurisdiction whose
/// deficit is nonzero, and the commit must satisfy what it answered.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "answer", rename_all = "snake_case")]
pub(crate) enum PatchAnswer {
    Boundary(super::CausalBoundary),
    Deficit(JurisdictionKey),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorldPatch {
    pub(crate) declarations: Vec<Declaration>,
    pub(crate) operations: Vec<ComponentOp>,
    pub(crate) evidence: Vec<EvidenceRef>,
}

/// One named structural check that a patch failed. A rejection carries the
/// complete set, never the first failure. It never enters `WorldState`,
/// `WorldEffect`, `CommandEnvelope`, or `WorldCommit`; the serde derives exist
/// for exactly one reader, the elaboration checkpoint, which is
/// controller-owned telemetry under its own row type and its own schema.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "mismatch", rename_all = "snake_case")]
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
    /// An `OfficeName`, `AuthorityKindName`, or `GrievanceKindName` that is not
    /// `[a-z][a-z0-9_]{0,47}`.
    InvalidCivicName {
        site: Site,
    },
    /// `OpenOffice` lending no authority kind.
    EmptyDelegation {
        operation: usize,
    },
    /// The institution named by an office operation is not
    /// `SubjectKind::Institution`.
    OfficeOnNonInstitution {
        operation: usize,
    },
    /// An incumbent that is not `SubjectKind::Person`. This is the check that
    /// keeps an institution's operational organ and its person-shaped voice two
    /// subjects joined by an office.
    OfficeHolderNotPerson {
        operation: usize,
    },
    /// This person already occupies another office of this institution.
    DuplicateIncumbency {
        operation: usize,
    },
    /// `InstallIncumbent`, `VacateOffice`, or `CloseOffice` naming an office the
    /// institution does not have.
    UnknownOffice {
        operation: usize,
    },
    /// `CloseForum` naming a grievance no forum takes.
    UnknownForum {
        operation: usize,
    },
    /// One subject would hold two grants of the same kind over overlapping
    /// targets, from any combination of direct grant and office delegation.
    /// Overlap between *different* subjects is layered government and legal:
    /// `Authorized` is a permission predicate, not an exclusivity claim, and
    /// this rule exists so nothing ever has to arbitrate between two sources.
    OverlappingJurisdiction {
        operation: usize,
    },
    /// A catalog entry declares a role named `actor`, which the kernel binds.
    ReservedRole {
        handle: DraftHandle,
    },
    /// An `EntityDeclaration` naming `Fact` or `Channel`. Those kinds carry a
    /// payload and have their own declaration; a payload-less one would make a
    /// referenceable, empty fact a legal state.
    PayloadEntityKind {
        handle: DraftHandle,
    },
    /// A `Canonical` fact declaration whose `EvidenceRef` is not listed in this
    /// patch's `evidence`.
    FactWithoutEvidence {
        handle: DraftHandle,
    },
    EmptyStatement {
        handle: DraftHandle,
    },
    /// `AcquireKnowledge { source: Evidenced }` over a `Claimed` fact.
    EvidencedKnowledgeOfClaim {
        operation: usize,
    },
    /// A `Communicate` whose speaker is neither inside the audience nor the
    /// channel's controller.
    SpeakerOutsideAudience {
        operation: usize,
    },
    /// A catalog entry with `carries_speech` and no `CanBroadcast` or
    /// `CanReach`: its speech would have no audience to lower.
    SpeechWithoutAudience {
        handle: DraftHandle,
    },
    /// More than one, so the lowering could not choose.
    AmbiguousSpeechAudience {
        handle: DraftHandle,
    },
    /// A `Routine` with no period, or a non-`Routine` with one.
    CommitmentPeriodMismatch {
        operation: usize,
    },
    /// A non-`Routine` carrying checks. Nothing would ever evaluate them.
    ChecksOnNonRoutine {
        operation: usize,
    },
    /// `due <= now` at creation. A promise cannot be born past due; it would
    /// press on its subject on the very next tick with no chance to act.
    CommitmentDueInThePast {
        operation: usize,
    },
    /// `counterparty == subject`.
    SelfCommitment {
        operation: usize,
    },
    /// A goal is a promise to oneself, and a promise to another is an
    /// obligation. This keeps the scale count's `Goal` clause meaning one thing.
    GoalWithCounterparty {
        operation: usize,
    },
    /// `DischargeCommitment` naming a key the subject does not hold.
    UnknownCommitment {
        operation: usize,
    },
    /// A scale-intent jurisdiction root that is not a declared place.
    UnknownJurisdictionRoot {
        handle: DraftHandle,
    },
    /// The permille weights sum over 1000: weights distribute the target and
    /// never raise it.
    ScaleWeightsExceedWhole,
    /// A jurisdictional author wrote outside its jurisdiction. The owner is
    /// unconfined and never sees this.
    OutsideJurisdiction {
        site: Site,
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

    pub(super) fn access(&self) -> &AccessKind {
        match self {
            Self::Route { access, .. } => access,
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

/// A declared fact: one `entities` row and one `facts` row, allocated together
/// so the bijection has one writer.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct ResolvedFact {
    pub(super) handle: DraftHandle,
    pub(super) entity_id: EntityId,
    pub(super) entity: EntityRecord,
    pub(super) fact: FactRecord,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct ResolvedChannel {
    pub(super) handle: DraftHandle,
    pub(super) entity_id: EntityId,
    pub(super) entity: EntityRecord,
    pub(super) channel: ChannelRecord,
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
    GrantAuthority {
        holder: SubjectId,
        grant: AuthorityGrant,
    },
    RevokeAuthority {
        holder: SubjectId,
        grant: AuthorityGrant,
    },
    OpenOffice {
        institution: SubjectId,
        office: OfficeName,
        delegated: BTreeSet<AuthorityKindName>,
    },
    CloseOffice {
        institution: SubjectId,
        office: OfficeName,
    },
    InstallIncumbent {
        institution: SubjectId,
        office: OfficeName,
        incumbent: SubjectId,
    },
    VacateOffice {
        institution: SubjectId,
        office: OfficeName,
    },
    OpenForum {
        grievance: GrievanceKindName,
        forum: SubjectId,
        standing: AuthorityTarget,
    },
    CloseForum {
        grievance: GrievanceKindName,
    },
    AcquireKnowledge {
        subject: SubjectId,
        fact: EntityId,
        source: AuthoredSource,
        confidence: Confidence,
    },
    /// Stores the audience, never the recipients. `apply_operation` re-derives
    /// the fan-out from live `positions`/`channels`, exactly as `Relocate`
    /// stores `edge_id` and re-derives the destination, so a forged effect
    /// cannot assert a landing the world does not have.
    Communicate {
        speaker: SubjectId,
        fact: EntityId,
        to: Audience,
    },
    Forget {
        subject: SubjectId,
        fact: EntityId,
    },
    SetReach {
        channel: EntityId,
        reach: Reach,
    },
    SetController {
        channel: EntityId,
        controller: Option<SubjectId>,
    },
    /// Kernel-only: no [`ComponentOp`] twin, because no proposer may author one.
    /// Synthesized by `action::exercise` for a speech-carrying invocation and by
    /// nothing else. Inserts the entity row and the `facts` row; it writes no
    /// knowledge, because a speaker's own knowledge of its claim is not implied.
    AssertClaim {
        fact: EntityId,
        statement: Statement,
        by: SubjectId,
    },
    CreateCommitment {
        subject: SubjectId,
        key: CommitmentKey,
        commitment: Commitment,
    },
    DischargeCommitment {
        subject: SubjectId,
        key: CommitmentKey,
    },
    AdvancePressure {
        source: PressureSource,
        target: SubjectId,
        by: PressureMagnitude,
    },
    ReducePressure {
        source: PressureSource,
        target: SubjectId,
        by: PressureMagnitude,
    },
    ResolvePressure {
        source: PressureSource,
        target: SubjectId,
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
    pub(super) facts: Vec<ResolvedFact>,
    pub(super) channels: Vec<ResolvedChannel>,
    pub(super) operations: Vec<ResolvedOp>,
    pub(super) evidence: Vec<EvidenceRef>,
    /// `Some` on the genesis lane and nowhere else: `CommandBody::AdmitPatch`
    /// carries no intent, so the write-once rule is the command shape rather
    /// than a check.
    pub(super) scale_intent: Option<WorldScaleIntent>,
}

impl ResolvedPatch {
    pub(super) fn declares_nothing(&self) -> bool {
        self.subjects.is_empty()
            && self.entities.is_empty()
            && self.routes.is_empty()
            && self.affordances.is_empty()
            && self.facts.is_empty()
            && self.channels.is_empty()
            && self.evidence.is_empty()
    }
}

const SUBJECT_NAMESPACE: &str = "ghostlight.id.subject.v1";
pub(super) const ENTITY_NAMESPACE: &str = "ghostlight.id.entity.v1";
const EDGE_NAMESPACE: &str = "ghostlight.id.edge.v1";
const CONTROLLER_NAMESPACE: &str = "ghostlight.id.controller.v1";
const AFFORDANCE_NAMESPACE: &str = "ghostlight.id.affordance.v1";

const PLACE: RefKind = RefKind::Entity(EntityKind::Place);
const FACT: RefKind = RefKind::Entity(EntityKind::Fact);
const CHANNEL: RefKind = RefKind::Entity(EntityKind::Channel);
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

/// [`PressureSource`] over candidate keys.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum PressureSourceKey {
    Commitment {
        subject: Key<SubjectId>,
        key: CommitmentKey,
    },
    Dependency(TargetKey),
    Subject(Key<SubjectId>),
}

/// [`AuthorityTarget`] over candidate keys, so a grant over a place this patch
/// declares is checked before any ID is minted.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum AuthorityTargetKey {
    Subject(Key<SubjectId>),
    PlaceSubtree(Key<EntityId>),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct GrantKey {
    kind: AuthorityKindName,
    over: AuthorityTargetKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OfficeCandidate {
    incumbent: Option<Key<SubjectId>>,
    delegated: BTreeSet<AuthorityKindName>,
}

/// Whether two jurisdictions of one kind cover common ground. Structural on
/// purpose: identity for named subordinates, containment for territory, and
/// never across the two, so the answer cannot decay when a subject walks
/// somewhere. Containment is fixed at declaration, so every way of creating an
/// overlap is a checked operation.
fn candidate_targets_overlap(
    left: &AuthorityTargetKey,
    right: &AuthorityTargetKey,
    containers: &BTreeMap<Key<EntityId>, Key<EntityId>>,
) -> bool {
    match (left, right) {
        (AuthorityTargetKey::Subject(one), AuthorityTargetKey::Subject(other)) => one == other,
        (AuthorityTargetKey::PlaceSubtree(one), AuthorityTargetKey::PlaceSubtree(other)) => {
            key_covers_place(one, other, containers) || key_covers_place(other, one, containers)
        }
        _ => false,
    }
}

/// Whether a candidate `Subject` grant target and a candidate `PlaceSubtree`
/// grant target of one kind name overlapping ground: a `PlaceSubtree` grant
/// covers a `Subject` grant's target when the subject's committed position
/// sits under that subtree. Checked live against `state` rather than the
/// candidate graph, because `covers` reads committed position and
/// containment, not anything a patch declares; a target still in this
/// patch's drafts has no committed position and so cannot overlap here. This
/// is the one cross-shape case `candidate_targets_overlap` leaves alone by
/// construction — same-shape overlap is its job, not this one's.
fn grant_targets_nest(
    state: &super::WorldState,
    left: &AuthorityTargetKey,
    right: &AuthorityTargetKey,
) -> bool {
    let place_and_subject = |a: &AuthorityTargetKey, b: &AuthorityTargetKey| match (a, b) {
        (
            AuthorityTargetKey::PlaceSubtree(Key::Existing(place)),
            AuthorityTargetKey::Subject(Key::Existing(subject_id)),
        ) => Some((*place, *subject_id)),
        _ => None,
    };
    place_and_subject(left, right)
        .or_else(|| place_and_subject(right, left))
        .is_some_and(|(place, subject_id)| {
            super::covers(
                state,
                AuthorityTarget::PlaceSubtree(place),
                super::Target::Subject(subject_id),
            )
        })
}

fn target_key_of(target: AuthorityTarget) -> AuthorityTargetKey {
    match target {
        AuthorityTarget::Subject(subject_id) => {
            AuthorityTargetKey::Subject(Key::Existing(subject_id))
        }
        AuthorityTarget::PlaceSubtree(entity_id) => {
            AuthorityTargetKey::PlaceSubtree(Key::Existing(entity_id))
        }
    }
}

fn grant_key_of(grant: &AuthorityGrant) -> GrantKey {
    GrantKey {
        kind: grant.kind.clone(),
        over: target_key_of(grant.over),
    }
}

/// The candidate authority of one subject, projected to the canonical referents
/// [`covers`](super::covers) reads. A grant naming structure this patch declares
/// cannot cover a place an already-canonical route reaches, so the projection
/// loses nothing the `Restricted` rule asks about.
fn canonical_grants(grants: &BTreeSet<GrantKey>) -> BTreeSet<AuthorityGrant> {
    grants
        .iter()
        .filter_map(|grant| {
            let over = match &grant.over {
                AuthorityTargetKey::Subject(Key::Existing(subject_id)) => {
                    AuthorityTarget::Subject(*subject_id)
                }
                AuthorityTargetKey::PlaceSubtree(Key::Existing(entity_id)) => {
                    AuthorityTarget::PlaceSubtree(*entity_id)
                }
                _ => return None,
            };
            Some(AuthorityGrant {
                kind: grant.kind.clone(),
                over,
            })
        })
        .collect()
}

/// Own grants plus what every held office lends, over the candidate graph. The
/// canonical twin is `scope_components` plus `effective_authority`; this one
/// answers the same question before any ID is minted.
fn candidate_effective_authority(
    holder: &Key<SubjectId>,
    authority: &BTreeMap<Key<SubjectId>, BTreeSet<GrantKey>>,
    selection: &BTreeMap<(Key<SubjectId>, OfficeName), OfficeCandidate>,
) -> BTreeSet<GrantKey> {
    let mut effective = authority.get(holder).cloned().unwrap_or_default();
    for ((institution, _), office) in selection {
        if office.incumbent.as_ref() != Some(holder) {
            continue;
        }
        for grant in authority.get(institution).into_iter().flatten() {
            if office.delegated.contains(&grant.kind) {
                effective.insert(grant.clone());
            }
        }
    }
    effective
}

/// A fact in the candidate graph. Only its standing matters to resolution: the
/// statement is never read and never compared.
#[derive(Clone, Debug, PartialEq, Eq)]
enum FactCandidate {
    Canonical,
    Claimed(Key<SubjectId>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ReachKey {
    Subjects(BTreeSet<Key<SubjectId>>),
    Place(Key<EntityId>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ChannelCandidate {
    reach: ReachKey,
    controller: Option<Key<SubjectId>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum KnowledgeSourceKey {
    Witnessed,
    Told {
        by: Key<SubjectId>,
        via: Option<Key<EntityId>>,
    },
    Evidenced,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct KnowledgeCandidate {
    confidence: Confidence,
    source: KnowledgeSourceKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AudienceKey {
    Colocated,
    Channel(Key<EntityId>),
}

/// Whether a subject is inside an audience over the candidate graph: the same
/// question `audience()` answers over committed state, asked before any ID is
/// minted. A controller is inside its own channel's audience for the purpose of
/// speaking on it — the horn belongs to the temple.
fn candidate_in_audience(
    subject: &Key<SubjectId>,
    audience: &AudienceKey,
    speaker: &Key<SubjectId>,
    positions: &BTreeMap<Key<SubjectId>, Key<EntityId>>,
    channels: &BTreeMap<Key<EntityId>, ChannelCandidate>,
    containers: &BTreeMap<Key<EntityId>, Key<EntityId>>,
) -> bool {
    match audience {
        AudienceKey::Colocated => match (positions.get(speaker), positions.get(subject)) {
            (Some(here), Some(there)) => here == there,
            _ => false,
        },
        AudienceKey::Channel(channel) => {
            let Some(record) = channels.get(channel) else {
                return false;
            };
            if record.controller.as_ref() == Some(subject) {
                return true;
            }
            match &record.reach {
                ReachKey::Subjects(members) => members.contains(subject),
                ReachKey::Place(root) => positions
                    .get(subject)
                    .is_some_and(|place| key_covers_place(root, place, containers)),
            }
        }
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

/// The only canonical ID allocator, called from exactly two sites:
/// [`resolve_patch`] and `action::exercise`. Deterministic on purpose: journal
/// replay recomputes `reduce` and requires effect equality, so a reduce arm can
/// never draw a `Uuid::new_v4`. Preimage fields are length-prefixed where they
/// are variable-width, so the concatenation is unambiguous — `discriminator`
/// included: a bare presence byte followed by a length prefix means
/// `Some("")` and `None` write different bytes, where an unprefixed
/// `Some("")` would have written nothing, same as `None`.
pub(super) fn derive_id(
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
    match discriminator {
        Some(discriminator) => {
            hasher.update([1u8]);
            hasher.update((discriminator.len() as u64).to_be_bytes());
            hasher.update(discriminator.as_bytes());
        }
        None => hasher.update([0u8]),
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

/// The one role name the kernel owns. A catalog entry may not declare it and an
/// invocation may not bind it: stage 2 binds it to the acting subject, so a slot
/// that must land on the actor — the payee of a levy, say — says so instead of
/// letting the proposer point it at a friend.
pub(super) const ACTOR_ROLE: &str = "actor";

/// Authority kinds, office names, and grievance kinds share the affordance
/// alphabet: world-declared, kernel-opaque, and safe to surface in a generated
/// tool description.
pub(super) fn is_civic_name(value: &str) -> bool {
    is_tool_name(value, 48)
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

/// Zero roles, zero slots, one empty band, one utterance, and one audience: a
/// voice fills the room it is standing in. Zero roles is load-bearing — the
/// Interpreter lane submits no bindings and Eve's speak payload carries only
/// text. The precondition is not vacuous: an unplaced subject has no
/// co-location audience and fails with `NoAudience`.
pub(super) fn kernel_speak_entry() -> Affordance {
    Affordance {
        kind: AffordanceKindName("speak".into()),
        roles: Vec::new(),
        preconditions: vec![Precondition::CanBroadcast {
            via: AudienceSpec::Colocated,
        }],
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
        if spec.role.0 == ACTOR_ROLE {
            mismatches.push(Mismatch::ReservedRole {
                handle: handle.clone(),
            });
        }
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
    // The kernel binds `actor` to the acting subject, so preconditions and
    // slots may name it like any role.
    declared.insert(Role(ACTOR_ROLE.into()), ANY_SUBJECT);
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
    let require_subject = |role: &Role, mismatches: &mut Vec<Mismatch>| match declared.get(role) {
        None => mismatches.push(Mismatch::UnknownRole {
            handle: handle.clone(),
            role: role.clone(),
        }),
        Some(RefKind::Subject(_)) => {}
        Some(_) => mismatches.push(Mismatch::RoleKindUnfit {
            handle: handle.clone(),
            role: role.clone(),
        }),
    };
    let require_audience = |via: &AudienceSpec, mismatches: &mut Vec<Mismatch>| match via {
        AudienceSpec::Colocated => {}
        AudienceSpec::Channel(role) => require(role, CHANNEL, mismatches),
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
            Precondition::Authorized { over, kind } => {
                // Jurisdiction runs over subjects, places, and routes. A
                // resource, fact, or channel is covered by nothing, so a role
                // of that kind is refused at declaration rather than always
                // failing at invocation.
                match declared.get(over) {
                    None => mismatches.push(Mismatch::UnknownRole {
                        handle: handle.clone(),
                        role: over.clone(),
                    }),
                    Some(RefKind::Subject(_)) | Some(&PLACE) | Some(&ROUTE) => {}
                    Some(_) => mismatches.push(Mismatch::RoleKindUnfit {
                        handle: handle.clone(),
                        role: over.clone(),
                    }),
                }
                if !is_civic_name(&kind.0) {
                    mismatches.push(Mismatch::InvalidCivicName { site: site() });
                }
            }
            Precondition::HasStanding { grievance } => {
                if !is_civic_name(&grievance.0) {
                    mismatches.push(Mismatch::InvalidCivicName { site: site() });
                }
            }
            Precondition::Knows { fact, .. } => require(fact, FACT, mismatches),
            Precondition::CanBroadcast { via } => require_audience(via, mismatches),
            Precondition::CanReach { subject, via } => {
                require_subject(subject, mismatches);
                require_audience(via, mismatches);
            }
            Precondition::Committed { to, .. } => require_subject(to, mismatches),
        }
    }
    // A speech-carrying entry must name exactly one audience: the lowering reads
    // it, so none is unlowerable and two is unchoosable.
    if carries_speech {
        let audiences = preconditions
            .iter()
            .filter(|precondition| {
                matches!(
                    precondition,
                    Precondition::CanBroadcast { .. } | Precondition::CanReach { .. }
                )
            })
            .count();
        if audiences == 0 {
            mismatches.push(Mismatch::SpeechWithoutAudience {
                handle: handle.clone(),
            });
        } else if audiences > 1 {
            mismatches.push(Mismatch::AmbiguousSpeechAudience {
                handle: handle.clone(),
            });
        }
    }
    for (index, slot) in effect_slots.iter().enumerate() {
        let arity = slot.op_kind.arity();
        if slot
            .op_kind
            .payload_names()
            .iter()
            .any(|name| !is_civic_name(name))
        {
            mismatches.push(Mismatch::InvalidCivicName { site: site() });
        }
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

/// Whether any subject in the candidate graph holds two grants of one kind over
/// overlapping ground, from any combination of direct grant and office
/// delegation.
fn graph_overlaps(
    authority: &BTreeMap<Key<SubjectId>, BTreeSet<GrantKey>>,
    selection: &BTreeMap<(Key<SubjectId>, OfficeName), OfficeCandidate>,
    containers: &BTreeMap<Key<EntityId>, Key<EntityId>>,
) -> bool {
    let holders: BTreeSet<&Key<SubjectId>> = authority
        .keys()
        .chain(
            selection
                .values()
                .filter_map(|office| office.incumbent.as_ref()),
        )
        .collect();
    holders.into_iter().any(|holder| {
        let effective: Vec<GrantKey> = candidate_effective_authority(holder, authority, selection)
            .into_iter()
            .collect();
        effective.iter().enumerate().any(|(index, one)| {
            effective[index + 1..].iter().any(|other| {
                one.kind == other.kind
                    && candidate_targets_overlap(&one.over, &other.over, containers)
            })
        })
    })
}

/// The candidate kind of a subject reference: the declared kind for a handle
/// this patch introduces, the committed kind otherwise.
fn subject_kind_of(
    key: &Key<SubjectId>,
    index: &BTreeMap<DraftHandle, RefKind>,
    state: &super::WorldState,
) -> Option<SubjectKind> {
    match key {
        Key::Existing(subject_id) => state.subjects.get(subject_id).map(|subject| subject.kind),
        Key::Draft(handle) => match index.get(handle) {
            Some(RefKind::Subject(kind)) => *kind,
            _ => None,
        },
    }
}

/// The institution half of every office operation: one subject reference, one
/// canonical office name, and the kind check that keeps an institution's
/// operational organ and its person-shaped voice two subjects.
fn resolve_office_institution(
    position: usize,
    institution: &Ref<SubjectId>,
    office: &OfficeName,
    index: &BTreeMap<DraftHandle, RefKind>,
    state: &super::WorldState,
    mismatches: &mut Vec<Mismatch>,
) -> Option<Key<SubjectId>> {
    let named = is_civic_name(&office.0);
    if !named {
        mismatches.push(Mismatch::InvalidCivicName {
            site: Site::Operation(position),
        });
    }
    let key = resolve_subject(
        Site::Operation(position),
        institution,
        index,
        &state.subjects,
        mismatches,
    )?;
    if subject_kind_of(&key, index, state) != Some(SubjectKind::Institution) {
        mismatches.push(Mismatch::OfficeOnNonInstitution {
            operation: position,
        });
        return None;
    }
    named.then_some(key)
}

/// The one reach resolver: every member of a subject set, or one place.
fn resolve_reach(
    site: Site,
    reach: &ReachRef,
    index: &BTreeMap<DraftHandle, RefKind>,
    state: &super::WorldState,
    mismatches: &mut Vec<Mismatch>,
) -> Option<ReachKey> {
    match reach {
        ReachRef::Subjects(members) => {
            let mut resolved = BTreeSet::new();
            let mut complete = true;
            for reference in members {
                match resolve_subject(site.clone(), reference, index, &state.subjects, mismatches) {
                    Some(key) => {
                        resolved.insert(key);
                    }
                    None => complete = false,
                }
            }
            complete.then_some(ReachKey::Subjects(resolved))
        }
        ReachRef::Place(reference) => resolve_entity(
            site,
            EntityKind::Place,
            reference,
            index,
            &state.entities,
            mismatches,
        )
        .map(ReachKey::Place),
    }
}

/// A channel controller is optional, so "declared none" and "named one that did
/// not resolve" are two answers, not one.
fn resolve_controller(
    site: Site,
    controller: &Option<Ref<SubjectId>>,
    index: &BTreeMap<DraftHandle, RefKind>,
    state: &super::WorldState,
    mismatches: &mut Vec<Mismatch>,
) -> Option<Option<Key<SubjectId>>> {
    match controller {
        None => Some(None),
        Some(reference) => {
            resolve_subject(site, reference, index, &state.subjects, mismatches).map(Some)
        }
    }
}

fn resolve_audience(
    site: Site,
    audience: &AudienceRef,
    index: &BTreeMap<DraftHandle, RefKind>,
    state: &super::WorldState,
    mismatches: &mut Vec<Mismatch>,
) -> Option<AudienceKey> {
    match audience {
        AudienceRef::Colocated => Some(AudienceKey::Colocated),
        AudienceRef::Channel(reference) => resolve_entity(
            site,
            EntityKind::Channel,
            reference,
            index,
            &state.entities,
            mismatches,
        )
        .map(AudienceKey::Channel),
    }
}

fn resolve_authority_target(
    site: Site,
    target: &AuthorityTargetRef,
    index: &BTreeMap<DraftHandle, RefKind>,
    state: &super::WorldState,
    mismatches: &mut Vec<Mismatch>,
) -> Option<AuthorityTargetKey> {
    match target {
        AuthorityTargetRef::Subject(reference) => {
            resolve_subject(site, reference, index, &state.subjects, mismatches)
                .map(AuthorityTargetKey::Subject)
        }
        AuthorityTargetRef::PlaceSubtree(reference) => resolve_entity(
            site,
            EntityKind::Place,
            reference,
            index,
            &state.entities,
            mismatches,
        )
        .map(AuthorityTargetKey::PlaceSubtree),
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
    scale_intent: Option<&WorldScaleIntentRef>,
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
            Declaration::Fact(fact) => (&fact.handle, Some(&fact.label), FACT),
            Declaration::Channel(channel) => (&channel.handle, Some(&channel.label), CHANNEL),
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
        if let Declaration::Entity(entity) = declaration
            && matches!(entity.kind, EntityKind::Fact | EntityKind::Channel)
        {
            mismatches.push(Mismatch::PayloadEntityKind {
                handle: handle.clone(),
            });
        }
        if let Declaration::Fact(fact) = declaration {
            if !is_canonical_text(fact.statement.as_str()) {
                mismatches.push(Mismatch::EmptyStatement {
                    handle: handle.clone(),
                });
            }
            // The same predicate `Admit` uses, moved into the declaration loop so
            // one rule serves evidenced quantity and evidenced canon alike.
            if let FactStandingRef::Canonical { evidence } = &fact.standing
                && !(is_canonical_text(&evidence.0) && patch.evidence.contains(evidence))
            {
                mismatches.push(Mismatch::FactWithoutEvidence {
                    handle: handle.clone(),
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
                    access: record.access().clone(),
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
    let mut authority: BTreeMap<Key<SubjectId>, BTreeSet<GrantKey>> = state
        .authority
        .iter()
        .map(|(subject_id, grants)| {
            (
                Key::Existing(*subject_id),
                grants.iter().map(grant_key_of).collect(),
            )
        })
        .collect();
    let mut selection: BTreeMap<(Key<SubjectId>, OfficeName), OfficeCandidate> = state
        .selection
        .iter()
        .flat_map(|(institution, offices)| {
            offices.iter().map(move |(name, office)| {
                (
                    (Key::Existing(*institution), name.clone()),
                    OfficeCandidate {
                        incumbent: office.incumbent.map(Key::Existing),
                        delegated: office.delegated.clone(),
                    },
                )
            })
        })
        .collect();
    let mut redress: BTreeMap<GrievanceKindName, (Key<SubjectId>, AuthorityTargetKey)> = state
        .redress
        .iter()
        .map(|(grievance, forum)| {
            (
                grievance.clone(),
                (Key::Existing(forum.forum), target_key_of(forum.standing)),
            )
        })
        .collect();
    let mut facts: BTreeMap<Key<EntityId>, FactCandidate> = state
        .facts
        .iter()
        .map(|(entity_id, record)| {
            (
                Key::Existing(*entity_id),
                match &record.standing {
                    FactStanding::Canonical { .. } => FactCandidate::Canonical,
                    FactStanding::Claimed { by } => FactCandidate::Claimed(Key::Existing(*by)),
                },
            )
        })
        .collect();
    let mut channels: BTreeMap<Key<EntityId>, ChannelCandidate> = state
        .channels
        .iter()
        .map(|(entity_id, record)| {
            (
                Key::Existing(*entity_id),
                ChannelCandidate {
                    reach: match &record.reach {
                        Reach::Subjects(members) => {
                            ReachKey::Subjects(members.iter().copied().map(Key::Existing).collect())
                        }
                        Reach::Place(place) => ReachKey::Place(Key::Existing(*place)),
                    },
                    controller: record.controller.map(Key::Existing),
                },
            )
        })
        .collect();
    // Keys and payload, so an `AcquireKnowledge` that changes nothing is
    // `NoOperationEffect` before any ID is minted. A `Communicate` never enters:
    // its fan-out is re-derived at apply, and a telling is a canonical change
    // whatever the room holds.
    //
    // This candidate map does not model `Communicate` at all — it has no case
    // that inserts a `Told` entry into it, so it cannot answer "does this
    // patch already know what a pending telling would land." That is safe
    // today only because no `ComponentOp` can construct a `KnowledgeSource`
    // carrying `Told`: `AcquireKnowledge` takes an `AuthoredSource`, and that
    // type has exactly two variants, `Witnessed` and `Evidenced` — `Told` is
    // unrepresentable there by construction (see `AuthoredSource`'s own doc
    // comment), and `action::no_component_op_kind_lowers_to_a_told_knowledge_write`
    // pins the one lowering site that could otherwise drift. If a future
    // operation is ever given the power to write `Told` directly — bypassing
    // `Communicate`'s own apply-time fan-out — it must model that fan-out into
    // this candidate graph first, or `resolve_patch` will silently reason
    // about a knowledge state the apply pass will not produce.
    let mut knowledge: BTreeMap<(Key<SubjectId>, Key<EntityId>), KnowledgeCandidate> = state
        .knowledge
        .iter()
        .flat_map(|(subject_id, held)| {
            held.iter().map(move |(fact, entry)| {
                (
                    (Key::Existing(*subject_id), Key::Existing(*fact)),
                    KnowledgeCandidate {
                        confidence: entry.confidence,
                        source: match entry.source {
                            KnowledgeSource::Witnessed => KnowledgeSourceKey::Witnessed,
                            KnowledgeSource::Evidenced => KnowledgeSourceKey::Evidenced,
                            KnowledgeSource::Told { by, via } => KnowledgeSourceKey::Told {
                                by: Key::Existing(by),
                                via: via.map(Key::Existing),
                            },
                        },
                    },
                )
            })
        })
        .collect();
    // Presence is all a discharge asks, and a create cannot collide: the key is
    // command-derived and the index is unique within one patch.
    let mut commitments: BTreeSet<(Key<SubjectId>, CommitmentKey)> = state
        .commitments
        .iter()
        .flat_map(|(subject_id, held)| {
            held.keys()
                .map(move |key| (Key::Existing(*subject_id), *key))
        })
        .collect();
    let mut pressures: BTreeMap<(Key<SubjectId>, PressureSourceKey), u32> = state
        .pressures
        .iter()
        .flat_map(|(target, held)| {
            held.iter().map(move |(source, magnitude)| {
                (
                    (
                        Key::Existing(*target),
                        match source {
                            PressureSource::Commitment { subject, key } => {
                                PressureSourceKey::Commitment {
                                    subject: Key::Existing(*subject),
                                    key: *key,
                                }
                            }
                            PressureSource::Dependency(DependencyTarget::Resource(entity_id)) => {
                                PressureSourceKey::Dependency(TargetKey::Resource(Key::Existing(
                                    *entity_id,
                                )))
                            }
                            PressureSource::Dependency(DependencyTarget::Route(edge_id)) => {
                                PressureSourceKey::Dependency(TargetKey::Route(Key::Existing(
                                    *edge_id,
                                )))
                            }
                            PressureSource::Dependency(DependencyTarget::Subject(subject_id)) => {
                                PressureSourceKey::Dependency(TargetKey::Subject(Key::Existing(
                                    *subject_id,
                                )))
                            }
                            PressureSource::Subject(subject_id) => {
                                PressureSourceKey::Subject(Key::Existing(*subject_id))
                            }
                        },
                    ),
                    magnitude.0,
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
            Declaration::Fact(fact) => {
                let standing = match &fact.standing {
                    FactStandingRef::Canonical { .. } => Some(FactCandidate::Canonical),
                    FactStandingRef::Claimed { by } => resolve_subject(
                        Site::Declaration(fact.handle.clone()),
                        by,
                        &index,
                        &state.subjects,
                        &mut mismatches,
                    )
                    .map(FactCandidate::Claimed),
                };
                if let Some(standing) = standing {
                    facts.insert(Key::Draft(fact.handle.clone()), standing);
                }
            }
            Declaration::Channel(channel) => {
                let reach = resolve_reach(
                    Site::Declaration(channel.handle.clone()),
                    &channel.reach,
                    &index,
                    state,
                    &mut mismatches,
                );
                let controller = resolve_controller(
                    Site::Declaration(channel.handle.clone()),
                    &channel.controller,
                    &index,
                    state,
                    &mut mismatches,
                );
                if let (Some(reach), Some(controller)) = (reach, controller) {
                    channels.insert(
                        Key::Draft(channel.handle.clone()),
                        ChannelCandidate { reach, controller },
                    );
                }
            }
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
                            access: route.access.clone(),
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
        let site = Site::Operation(position);
        match operation {
            ComponentOp::CreateCommitment {
                subject,
                counterparty,
                kind,
                due,
                period,
                checks,
            } => {
                let subject_key = resolve_subject(
                    site.clone(),
                    subject,
                    &index,
                    &state.subjects,
                    &mut mismatches,
                );
                let counterparty_key = counterparty.as_ref().map(|reference| {
                    resolve_subject(
                        site.clone(),
                        reference,
                        &index,
                        &state.subjects,
                        &mut mismatches,
                    )
                });
                let mut checks_resolve = true;
                for check in checks {
                    checks_resolve &=
                        resolve_check(site.clone(), check, &index, state, &mut mismatches);
                }
                if (*kind == CommitmentKind::Routine) != period.is_some() {
                    mismatches.push(Mismatch::CommitmentPeriodMismatch {
                        operation: position,
                    });
                }
                if let Some(period) = period
                    && !is_valid_cost(Cost(period.minutes()))
                {
                    mismatches.push(Mismatch::InvalidCost { site: site.clone() });
                }
                if *kind != CommitmentKind::Routine && !checks.is_empty() {
                    mismatches.push(Mismatch::ChecksOnNonRoutine {
                        operation: position,
                    });
                }
                // A promise cannot be born past due: it would press on its
                // subject on the very next tick with no chance to act.
                if *due <= state.now {
                    mismatches.push(Mismatch::CommitmentDueInThePast {
                        operation: position,
                    });
                }
                if counterparty.is_some() && *kind == CommitmentKind::Goal {
                    mismatches.push(Mismatch::GoalWithCounterparty {
                        operation: position,
                    });
                }
                let Some(subject_key) = subject_key else {
                    continue;
                };
                if let Some(counterparty_key) = &counterparty_key {
                    match counterparty_key {
                        None => continue,
                        Some(counterparty_key) if *counterparty_key == subject_key => {
                            mismatches.push(Mismatch::SelfCommitment {
                                operation: position,
                            });
                            continue;
                        }
                        Some(_) => {}
                    }
                }
                if !checks_resolve {
                    continue;
                }
                commitments.insert((
                    subject_key,
                    CommitmentKey {
                        command: command_id,
                        index: position as u32,
                    },
                ));
            }
            ComponentOp::DischargeCommitment { subject, key } => {
                let Some(subject_key) = resolve_subject(
                    site.clone(),
                    subject,
                    &index,
                    &state.subjects,
                    &mut mismatches,
                ) else {
                    continue;
                };
                if !commitments.remove(&(subject_key, *key)) {
                    mismatches.push(Mismatch::UnknownCommitment {
                        operation: position,
                    });
                }
            }
            ComponentOp::AdvancePressure { source, target, by }
            | ComponentOp::ReducePressure { source, target, by } => {
                let advancing = matches!(operation, ComponentOp::AdvancePressure { .. });
                let target_key = resolve_subject(
                    site.clone(),
                    target,
                    &index,
                    &state.subjects,
                    &mut mismatches,
                );
                let source_key =
                    resolve_pressure_source(site.clone(), source, &index, state, &mut mismatches);
                let (Some(target_key), Some(source_key)) = (target_key, source_key) else {
                    continue;
                };
                let slot = (target_key, source_key);
                let current = pressures.get(&slot).copied().unwrap_or_default();
                let next = if advancing {
                    current.saturating_add(by.0)
                } else {
                    current.saturating_sub(by.0)
                };
                // A zero step and a subtraction that hits a floor both change
                // nothing, which is one name and not three.
                if next == current {
                    mismatches.push(Mismatch::NoOperationEffect {
                        operation: position,
                    });
                } else if next == 0 {
                    pressures.remove(&slot);
                } else {
                    pressures.insert(slot, next);
                }
            }
            ComponentOp::ResolvePressure { source, target } => {
                let target_key = resolve_subject(
                    site.clone(),
                    target,
                    &index,
                    &state.subjects,
                    &mut mismatches,
                );
                let source_key =
                    resolve_pressure_source(site.clone(), source, &index, state, &mut mismatches);
                let (Some(target_key), Some(source_key)) = (target_key, source_key) else {
                    continue;
                };
                if pressures.remove(&(target_key, source_key)).is_none() {
                    mismatches.push(Mismatch::NoOperationEffect {
                        operation: position,
                    });
                }
            }
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
                // The resolver reads authority for the subject being moved, not
                // for whoever proposed the patch, which is what keeps it
                // actor-blind: the owner-admitted lane and the action lane get
                // the same answer for the same move. A destination this patch
                // declares is canonically unknown, so only a `Public` route
                // reaches it.
                let opened = match &route.to {
                    Key::Existing(destination) => route_admits(
                        state,
                        &canonical_grants(&candidate_effective_authority(
                            &subject_key,
                            &authority,
                            &selection,
                        )),
                        &route.access,
                        *destination,
                    ),
                    Key::Draft(_) => route.access == AccessKind::Public,
                };
                if !opened {
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
            ComponentOp::GrantAuthority { holder, grant }
            | ComponentOp::RevokeAuthority { holder, grant } => {
                let granting = matches!(operation, ComponentOp::GrantAuthority { .. });
                let holder_key = resolve_subject(
                    Site::Operation(position),
                    holder,
                    &index,
                    &state.subjects,
                    &mut mismatches,
                );
                if !is_civic_name(&grant.kind.0) {
                    mismatches.push(Mismatch::InvalidCivicName {
                        site: Site::Operation(position),
                    });
                }
                let over = resolve_authority_target(
                    Site::Operation(position),
                    &grant.over,
                    &index,
                    state,
                    &mut mismatches,
                );
                let (Some(holder_key), Some(over)) = (holder_key, over) else {
                    continue;
                };
                let entry = GrantKey {
                    kind: grant.kind.clone(),
                    over,
                };
                // Admission-time only: a `Subject` grant landing inside a
                // `PlaceSubtree` grant the holder already has, or the reverse,
                // is a second source for the same kind of act. This does not
                // reach `targets_overlap`/`verify_state_shape`, which stay
                // structural and position-independent; a later `Relocate`
                // that creates this shape is not re-derived as a violation.
                if granting
                    && authority.get(&holder_key).is_some_and(|grants| {
                        grants.iter().any(|other| {
                            other.kind == entry.kind
                                && grant_targets_nest(state, &entry.over, &other.over)
                        })
                    })
                {
                    mismatches.push(Mismatch::OverlappingJurisdiction {
                        operation: position,
                    });
                    continue;
                }
                let held = authority
                    .get(&holder_key)
                    .is_some_and(|grants| grants.contains(&entry));
                if held == granting {
                    mismatches.push(Mismatch::NoOperationEffect {
                        operation: position,
                    });
                } else if granting {
                    authority.entry(holder_key).or_default().insert(entry);
                } else {
                    let empty = if let Some(grants) = authority.get_mut(&holder_key) {
                        grants.remove(&entry);
                        grants.is_empty()
                    } else {
                        false
                    };
                    if empty {
                        authority.remove(&holder_key);
                    }
                }
            }
            ComponentOp::OpenOffice {
                institution,
                office,
                delegated,
            } => {
                let institution_key = resolve_office_institution(
                    position,
                    institution,
                    office,
                    &index,
                    state,
                    &mut mismatches,
                );
                if delegated.is_empty() {
                    mismatches.push(Mismatch::EmptyDelegation {
                        operation: position,
                    });
                }
                if delegated.iter().any(|kind| !is_civic_name(&kind.0)) {
                    mismatches.push(Mismatch::InvalidCivicName {
                        site: Site::Operation(position),
                    });
                }
                let Some(institution_key) = institution_key else {
                    continue;
                };
                if delegated.is_empty() {
                    continue;
                }
                let slot = (institution_key, office.clone());
                let incumbent = selection
                    .get(&slot)
                    .and_then(|current| current.incumbent.clone());
                if selection
                    .get(&slot)
                    .is_some_and(|current| &current.delegated == delegated)
                {
                    mismatches.push(Mismatch::NoOperationEffect {
                        operation: position,
                    });
                } else {
                    selection.insert(
                        slot,
                        OfficeCandidate {
                            incumbent,
                            delegated: delegated.clone(),
                        },
                    );
                }
            }
            ComponentOp::CloseOffice {
                institution,
                office,
            }
            | ComponentOp::VacateOffice {
                institution,
                office,
            } => {
                let closing = matches!(operation, ComponentOp::CloseOffice { .. });
                let Some(institution_key) = resolve_office_institution(
                    position,
                    institution,
                    office,
                    &index,
                    state,
                    &mut mismatches,
                ) else {
                    continue;
                };
                let slot = (institution_key, office.clone());
                let Some(current) = selection.get(&slot).cloned() else {
                    mismatches.push(Mismatch::UnknownOffice {
                        operation: position,
                    });
                    continue;
                };
                if closing {
                    selection.remove(&slot);
                } else if current.incumbent.is_none() {
                    mismatches.push(Mismatch::NoOperationEffect {
                        operation: position,
                    });
                } else {
                    selection.insert(
                        slot,
                        OfficeCandidate {
                            incumbent: None,
                            ..current
                        },
                    );
                }
            }
            ComponentOp::InstallIncumbent {
                institution,
                office,
                incumbent,
            } => {
                let institution_key = resolve_office_institution(
                    position,
                    institution,
                    office,
                    &index,
                    state,
                    &mut mismatches,
                );
                let incumbent_key = resolve_subject(
                    Site::Operation(position),
                    incumbent,
                    &index,
                    &state.subjects,
                    &mut mismatches,
                );
                if let Some(incumbent_key) = &incumbent_key
                    && subject_kind_of(incumbent_key, &index, state) != Some(SubjectKind::Person)
                {
                    mismatches.push(Mismatch::OfficeHolderNotPerson {
                        operation: position,
                    });
                    continue;
                }
                let (Some(institution_key), Some(incumbent_key)) = (institution_key, incumbent_key)
                else {
                    continue;
                };
                let slot = (institution_key.clone(), office.clone());
                let Some(current) = selection.get(&slot).cloned() else {
                    mismatches.push(Mismatch::UnknownOffice {
                        operation: position,
                    });
                    continue;
                };
                if selection.iter().any(|((held_by, name), other)| {
                    *held_by == institution_key
                        && name != office
                        && other.incumbent.as_ref() == Some(&incumbent_key)
                }) {
                    mismatches.push(Mismatch::DuplicateIncumbency {
                        operation: position,
                    });
                    continue;
                }
                if current.incumbent.as_ref() == Some(&incumbent_key) {
                    mismatches.push(Mismatch::NoOperationEffect {
                        operation: position,
                    });
                } else {
                    selection.insert(
                        slot,
                        OfficeCandidate {
                            incumbent: Some(incumbent_key),
                            ..current
                        },
                    );
                }
            }
            ComponentOp::OpenForum {
                grievance,
                forum,
                standing,
            } => {
                if !is_civic_name(&grievance.0) {
                    mismatches.push(Mismatch::InvalidCivicName {
                        site: Site::Operation(position),
                    });
                }
                let forum_key = resolve_subject(
                    Site::Operation(position),
                    forum,
                    &index,
                    &state.subjects,
                    &mut mismatches,
                );
                let standing_key = resolve_authority_target(
                    Site::Operation(position),
                    standing,
                    &index,
                    state,
                    &mut mismatches,
                );
                let (Some(forum_key), Some(standing_key)) = (forum_key, standing_key) else {
                    continue;
                };
                if redress.get(grievance) == Some(&(forum_key.clone(), standing_key.clone())) {
                    mismatches.push(Mismatch::NoOperationEffect {
                        operation: position,
                    });
                } else {
                    redress.insert(grievance.clone(), (forum_key, standing_key));
                }
            }
            ComponentOp::CloseForum { grievance } => {
                if redress.remove(grievance).is_none() {
                    mismatches.push(Mismatch::UnknownForum {
                        operation: position,
                    });
                }
            }
            ComponentOp::AcquireKnowledge {
                subject,
                fact,
                source,
                confidence,
            } => {
                let subject_key = resolve_subject(
                    Site::Operation(position),
                    subject,
                    &index,
                    &state.subjects,
                    &mut mismatches,
                );
                let fact_key = resolve_entity(
                    Site::Operation(position),
                    EntityKind::Fact,
                    fact,
                    &index,
                    &state.entities,
                    &mut mismatches,
                );
                let (Some(subject_key), Some(fact_key)) = (subject_key, fact_key) else {
                    continue;
                };
                // Evidenced knowledge is knowledge of canon. A receipt cannot
                // vouch for an assertion the kernel never evaluated.
                if *source == AuthoredSource::Evidenced
                    && facts.get(&fact_key) != Some(&FactCandidate::Canonical)
                {
                    mismatches.push(Mismatch::EvidencedKnowledgeOfClaim {
                        operation: position,
                    });
                    continue;
                }
                let entry = KnowledgeCandidate {
                    confidence: *confidence,
                    source: match source {
                        AuthoredSource::Witnessed => KnowledgeSourceKey::Witnessed,
                        AuthoredSource::Evidenced => KnowledgeSourceKey::Evidenced,
                    },
                };
                let slot = (subject_key, fact_key);
                if knowledge.get(&slot) == Some(&entry) {
                    mismatches.push(Mismatch::NoOperationEffect {
                        operation: position,
                    });
                } else {
                    knowledge.insert(slot, entry);
                }
            }
            ComponentOp::Forget { subject, fact } => {
                let subject_key = resolve_subject(
                    Site::Operation(position),
                    subject,
                    &index,
                    &state.subjects,
                    &mut mismatches,
                );
                let fact_key = resolve_entity(
                    Site::Operation(position),
                    EntityKind::Fact,
                    fact,
                    &index,
                    &state.entities,
                    &mut mismatches,
                );
                let (Some(subject_key), Some(fact_key)) = (subject_key, fact_key) else {
                    continue;
                };
                if knowledge.remove(&(subject_key, fact_key)).is_none() {
                    mismatches.push(Mismatch::NoOperationEffect {
                        operation: position,
                    });
                }
            }
            ComponentOp::Communicate { speaker, fact, to } => {
                let speaker_key = resolve_subject(
                    Site::Operation(position),
                    speaker,
                    &index,
                    &state.subjects,
                    &mut mismatches,
                );
                let fact_key = resolve_entity(
                    Site::Operation(position),
                    EntityKind::Fact,
                    fact,
                    &index,
                    &state.entities,
                    &mut mismatches,
                );
                let audience = resolve_audience(
                    Site::Operation(position),
                    to,
                    &index,
                    state,
                    &mut mismatches,
                );
                let (Some(speaker_key), Some(_), Some(audience)) =
                    (speaker_key, fact_key, audience)
                else {
                    continue;
                };
                // Ontology admission: communication reaches only subjects inside
                // the channel's reach, and a speaker outside it is not speaking.
                if !candidate_in_audience(
                    &speaker_key,
                    &audience,
                    &speaker_key,
                    &positions,
                    &channels,
                    &containers,
                ) {
                    mismatches.push(Mismatch::SpeakerOutsideAudience {
                        operation: position,
                    });
                }
            }
            ComponentOp::SetReach { channel, reach } => {
                let channel_key = resolve_entity(
                    Site::Operation(position),
                    EntityKind::Channel,
                    channel,
                    &index,
                    &state.entities,
                    &mut mismatches,
                );
                let reach_key = resolve_reach(
                    Site::Operation(position),
                    reach,
                    &index,
                    state,
                    &mut mismatches,
                );
                let (Some(channel_key), Some(reach_key)) = (channel_key, reach_key) else {
                    continue;
                };
                let Some(candidate) = channels.get_mut(&channel_key) else {
                    continue;
                };
                if candidate.reach == reach_key {
                    mismatches.push(Mismatch::NoOperationEffect {
                        operation: position,
                    });
                } else {
                    candidate.reach = reach_key;
                }
            }
            ComponentOp::SetController {
                channel,
                controller,
            } => {
                let channel_key = resolve_entity(
                    Site::Operation(position),
                    EntityKind::Channel,
                    channel,
                    &index,
                    &state.entities,
                    &mut mismatches,
                );
                let controller_key = resolve_controller(
                    Site::Operation(position),
                    controller,
                    &index,
                    state,
                    &mut mismatches,
                );
                let (Some(channel_key), Some(controller_key)) = (channel_key, controller_key)
                else {
                    continue;
                };
                let Some(candidate) = channels.get_mut(&channel_key) else {
                    continue;
                };
                if candidate.controller == controller_key {
                    mismatches.push(Mismatch::NoOperationEffect {
                        operation: position,
                    });
                } else {
                    candidate.controller = controller_key;
                }
            }
        }
        // Disjointness, checked against the complete candidate graph after each
        // operation that could widen someone's jurisdiction, so two operations
        // in one patch collide with each other and the failure names the one
        // that created the collision. Only three operations can: a new grant, a
        // widened office, and a new incumbency.
        if matches!(
            operation,
            ComponentOp::GrantAuthority { .. }
                | ComponentOp::OpenOffice { .. }
                | ComponentOp::InstallIncumbent { .. }
        ) && graph_overlaps(&authority, &selection, &containers)
        {
            mismatches.push(Mismatch::OverlappingJurisdiction {
                operation: position,
            });
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

    if let Some(intent) = scale_intent {
        for handle in intent.jurisdictions.keys() {
            if index.get(handle) != Some(&PLACE) {
                mismatches.push(Mismatch::UnknownJurisdictionRoot {
                    handle: handle.clone(),
                });
            }
        }
        // Weights distribute the target and never raise it.
        if intent
            .jurisdictions
            .values()
            .map(|weight| u64::from(*weight))
            .sum::<u64>()
            > 1000
        {
            mismatches.push(Mismatch::ScaleWeightsExceedWhole);
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
    // Facts and channels allocate through the same entity namespace and the same
    // `derive_id`, so no namespace is added and `RefKind::Entity` types every
    // reference to one. They stay out of `entities` above so that vector remains
    // a zip over the place and resource declarations.
    for declaration in &patch.declarations {
        let handle = match declaration {
            Declaration::Fact(fact) => &fact.handle,
            Declaration::Channel(channel) => &channel.handle,
            _ => continue,
        };
        let entity_id = EntityId(derive_id(
            ENTITY_NAMESPACE,
            world_id,
            command_id,
            handle,
            None,
        ));
        if state.entities.contains_key(&entity_id) {
            collisions.push(Mismatch::CanonicalCollision {
                handle: handle.clone(),
            });
        }
        allocated_entities.insert(handle.clone(), entity_id);
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
                access: route.access.clone(),
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
    let authority_target_of = |target: &AuthorityTargetRef| -> AuthorityTarget {
        match target {
            AuthorityTargetRef::Subject(reference) => {
                AuthorityTarget::Subject(subject_id_of(&key_of(reference)))
            }
            AuthorityTargetRef::PlaceSubtree(reference) => {
                AuthorityTarget::PlaceSubtree(entity_id_of(&key_of(reference)))
            }
        }
    };
    let reach_of = |reach: &ReachRef| -> Reach {
        match reach {
            ReachRef::Subjects(members) => Reach::Subjects(
                members
                    .iter()
                    .map(|reference| subject_id_of(&key_of(reference)))
                    .collect(),
            ),
            ReachRef::Place(reference) => Reach::Place(entity_id_of(&key_of(reference))),
        }
    };
    let controller_of = |controller: &Option<Ref<SubjectId>>| -> Option<SubjectId> {
        controller
            .as_ref()
            .map(|reference| subject_id_of(&key_of(reference)))
    };
    let audience_of = |audience: &AudienceRef| -> Audience {
        match audience {
            AudienceRef::Colocated => Audience::Colocated,
            AudienceRef::Channel(reference) => Audience::Channel(entity_id_of(&key_of(reference))),
        }
    };
    let mut declared_facts = Vec::new();
    let mut declared_channels = Vec::new();
    for declaration in &patch.declarations {
        match declaration {
            Declaration::Fact(fact) => declared_facts.push(ResolvedFact {
                handle: fact.handle.clone(),
                entity_id: entity_id_of(&Key::Draft(fact.handle.clone())),
                entity: EntityRecord {
                    label: fact.label.clone(),
                    kind: EntityKind::Fact,
                    container: None,
                },
                fact: FactRecord {
                    statement: fact.statement.clone(),
                    standing: match &fact.standing {
                        FactStandingRef::Canonical { evidence } => FactStanding::Canonical {
                            evidence: evidence.clone(),
                        },
                        FactStandingRef::Claimed { by } => FactStanding::Claimed {
                            by: subject_id_of(&key_of(by)),
                        },
                    },
                },
            }),
            Declaration::Channel(channel) => declared_channels.push(ResolvedChannel {
                handle: channel.handle.clone(),
                entity_id: entity_id_of(&Key::Draft(channel.handle.clone())),
                entity: EntityRecord {
                    label: channel.label.clone(),
                    kind: EntityKind::Channel,
                    container: None,
                },
                channel: ChannelRecord {
                    reach: reach_of(&channel.reach),
                    controller: controller_of(&channel.controller),
                },
            }),
            _ => {}
        }
    }
    let grant_of = |grant: &AuthorityGrantRef| -> AuthorityGrant {
        AuthorityGrant {
            kind: grant.kind.clone(),
            over: authority_target_of(&grant.over),
        }
    };
    let precondition_of = |check: &PreconditionRef| -> BoundPrecondition {
        match check {
            PreconditionRef::Present { at } => BoundPrecondition::Present {
                at: entity_id_of(&key_of(at)),
            },
            PreconditionRef::Reachable { to, within } => BoundPrecondition::Reachable {
                to: entity_id_of(&key_of(to)),
                within: *within,
            },
            PreconditionRef::Holds { resource, at_least } => BoundPrecondition::Holds {
                resource: entity_id_of(&key_of(resource)),
                at_least: *at_least,
            },
            PreconditionRef::Authorized { over, kind } => BoundPrecondition::Authorized {
                over: authority_target_of(over).as_referent(),
                kind: kind.clone(),
            },
            PreconditionRef::HasStanding { grievance } => BoundPrecondition::HasStanding {
                grievance: grievance.clone(),
            },
            PreconditionRef::Knows { fact, at_least } => BoundPrecondition::Knows {
                fact: entity_id_of(&key_of(fact)),
                at_least: *at_least,
            },
            PreconditionRef::CanBroadcast { via } => BoundPrecondition::CanBroadcast {
                via: audience_of(via),
            },
            PreconditionRef::CanReach { subject, via } => BoundPrecondition::CanReach {
                subject: subject_id_of(&key_of(subject)),
                via: audience_of(via),
            },
            PreconditionRef::Committed { to, kind } => BoundPrecondition::Committed {
                to: subject_id_of(&key_of(to)),
                kind: *kind,
            },
        }
    };
    let pressure_source_of = |source: &PressureSourceRef| -> PressureSource {
        match source {
            PressureSourceRef::Commitment { subject, key } => PressureSource::Commitment {
                subject: subject_id_of(&key_of(subject)),
                key: *key,
            },
            PressureSourceRef::Dependency(target) => PressureSource::Dependency(target_of(target)),
            PressureSourceRef::Subject(reference) => {
                PressureSource::Subject(subject_id_of(&key_of(reference)))
            }
        }
    };
    let operations = patch
        .operations
        .iter()
        .enumerate()
        .map(|(position, operation)| match operation {
            ComponentOp::CreateCommitment {
                subject,
                counterparty,
                kind,
                due,
                period,
                checks,
            } => ResolvedOp::CreateCommitment {
                subject: subject_id_of(&key_of(subject)),
                key: CommitmentKey {
                    command: command_id,
                    index: position as u32,
                },
                commitment: Commitment {
                    kind: *kind,
                    counterparty: counterparty
                        .as_ref()
                        .map(|reference| subject_id_of(&key_of(reference))),
                    due: *due,
                    period: *period,
                    checks: checks.iter().map(precondition_of).collect(),
                },
            },
            ComponentOp::DischargeCommitment { subject, key } => ResolvedOp::DischargeCommitment {
                subject: subject_id_of(&key_of(subject)),
                key: *key,
            },
            ComponentOp::AdvancePressure { source, target, by } => ResolvedOp::AdvancePressure {
                source: pressure_source_of(source),
                target: subject_id_of(&key_of(target)),
                by: *by,
            },
            ComponentOp::ReducePressure { source, target, by } => ResolvedOp::ReducePressure {
                source: pressure_source_of(source),
                target: subject_id_of(&key_of(target)),
                by: *by,
            },
            ComponentOp::ResolvePressure { source, target } => ResolvedOp::ResolvePressure {
                source: pressure_source_of(source),
                target: subject_id_of(&key_of(target)),
            },
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
            ComponentOp::GrantAuthority { holder, grant } => ResolvedOp::GrantAuthority {
                holder: subject_id_of(&key_of(holder)),
                grant: grant_of(grant),
            },
            ComponentOp::RevokeAuthority { holder, grant } => ResolvedOp::RevokeAuthority {
                holder: subject_id_of(&key_of(holder)),
                grant: grant_of(grant),
            },
            ComponentOp::OpenOffice {
                institution,
                office,
                delegated,
            } => ResolvedOp::OpenOffice {
                institution: subject_id_of(&key_of(institution)),
                office: office.clone(),
                delegated: delegated.clone(),
            },
            ComponentOp::CloseOffice {
                institution,
                office,
            } => ResolvedOp::CloseOffice {
                institution: subject_id_of(&key_of(institution)),
                office: office.clone(),
            },
            ComponentOp::InstallIncumbent {
                institution,
                office,
                incumbent,
            } => ResolvedOp::InstallIncumbent {
                institution: subject_id_of(&key_of(institution)),
                office: office.clone(),
                incumbent: subject_id_of(&key_of(incumbent)),
            },
            ComponentOp::VacateOffice {
                institution,
                office,
            } => ResolvedOp::VacateOffice {
                institution: subject_id_of(&key_of(institution)),
                office: office.clone(),
            },
            ComponentOp::OpenForum {
                grievance,
                forum,
                standing,
            } => ResolvedOp::OpenForum {
                grievance: grievance.clone(),
                forum: subject_id_of(&key_of(forum)),
                standing: authority_target_of(standing),
            },
            ComponentOp::CloseForum { grievance } => ResolvedOp::CloseForum {
                grievance: grievance.clone(),
            },
            ComponentOp::AcquireKnowledge {
                subject,
                fact,
                source,
                confidence,
            } => ResolvedOp::AcquireKnowledge {
                subject: subject_id_of(&key_of(subject)),
                fact: entity_id_of(&key_of(fact)),
                source: *source,
                confidence: *confidence,
            },
            ComponentOp::Communicate { speaker, fact, to } => ResolvedOp::Communicate {
                speaker: subject_id_of(&key_of(speaker)),
                fact: entity_id_of(&key_of(fact)),
                to: audience_of(to),
            },
            ComponentOp::Forget { subject, fact } => ResolvedOp::Forget {
                subject: subject_id_of(&key_of(subject)),
                fact: entity_id_of(&key_of(fact)),
            },
            ComponentOp::SetReach { channel, reach } => ResolvedOp::SetReach {
                channel: entity_id_of(&key_of(channel)),
                reach: reach_of(reach),
            },
            ComponentOp::SetController {
                channel,
                controller,
            } => ResolvedOp::SetController {
                channel: entity_id_of(&key_of(channel)),
                controller: controller_of(controller),
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
        facts: declared_facts,
        channels: declared_channels,
        operations,
        evidence: patch.evidence.clone(),
        scale_intent: scale_intent.map(|intent| WorldScaleIntent {
            targets: intent.targets.clone(),
            jurisdictions: intent
                .jurisdictions
                .iter()
                .map(|(handle, weight)| {
                    (
                        *allocated_entities
                            .get(handle)
                            .expect("a jurisdiction root resolved above"),
                        *weight,
                    )
                })
                .collect(),
        }),
    })
}

/// Whether every referent one stored check names resolves. The canonical
/// lowering runs after allocation, so a check that does not resolve here would
/// have nothing to lower to.
fn resolve_check(
    site: Site,
    check: &PreconditionRef,
    index: &BTreeMap<DraftHandle, RefKind>,
    state: &super::WorldState,
    mismatches: &mut Vec<Mismatch>,
) -> bool {
    let entity = |kind: EntityKind, reference: &Ref<EntityId>, mismatches: &mut Vec<Mismatch>| {
        resolve_entity(
            site.clone(),
            kind,
            reference,
            index,
            &state.entities,
            mismatches,
        )
        .is_some()
    };
    let audience = |via: &AudienceRef, mismatches: &mut Vec<Mismatch>| {
        resolve_audience(site.clone(), via, index, state, mismatches).is_some()
    };
    let subject = |reference: &Ref<SubjectId>, mismatches: &mut Vec<Mismatch>| {
        resolve_subject(site.clone(), reference, index, &state.subjects, mismatches).is_some()
    };
    match check {
        PreconditionRef::Present { at } => entity(EntityKind::Place, at, mismatches),
        PreconditionRef::Reachable { to, within } => {
            let resolved = entity(EntityKind::Place, to, mismatches);
            if !is_valid_cost(*within) {
                mismatches.push(Mismatch::InvalidCost { site: site.clone() });
                return false;
            }
            resolved
        }
        PreconditionRef::Holds { resource, .. } => {
            entity(EntityKind::Resource, resource, mismatches)
        }
        PreconditionRef::Authorized { over, kind } => {
            let resolved =
                resolve_authority_target(site.clone(), over, index, state, mismatches).is_some();
            if !is_civic_name(&kind.0) {
                mismatches.push(Mismatch::InvalidCivicName { site: site.clone() });
                return false;
            }
            resolved
        }
        PreconditionRef::HasStanding { grievance } => {
            if is_civic_name(&grievance.0) {
                true
            } else {
                mismatches.push(Mismatch::InvalidCivicName { site: site.clone() });
                false
            }
        }
        PreconditionRef::Knows { fact, .. } => entity(EntityKind::Fact, fact, mismatches),
        PreconditionRef::CanBroadcast { via } => audience(via, mismatches),
        PreconditionRef::CanReach {
            subject: named,
            via,
        } => subject(named, mismatches) & audience(via, mismatches),
        PreconditionRef::Committed { to, .. } => subject(to, mismatches),
    }
}

fn resolve_pressure_source(
    site: Site,
    source: &PressureSourceRef,
    index: &BTreeMap<DraftHandle, RefKind>,
    state: &super::WorldState,
    mismatches: &mut Vec<Mismatch>,
) -> Option<PressureSourceKey> {
    match source {
        PressureSourceRef::Commitment { subject, key } => {
            resolve_subject(site, subject, index, &state.subjects, mismatches)
                .map(|subject| PressureSourceKey::Commitment { subject, key: *key })
        }
        PressureSourceRef::Dependency(target) => {
            resolve_dependency_target(site, target, index, state, mismatches)
                .map(PressureSourceKey::Dependency)
        }
        PressureSourceRef::Subject(reference) => {
            resolve_subject(site, reference, index, &state.subjects, mismatches)
                .map(PressureSourceKey::Subject)
        }
    }
}

/// Whether `place` is `root`, or reaches it through the candidate container
/// chain. The walk is bounded by the graph size, so a cycle that does not
/// include either end still terminates.
fn key_covers_place(
    root: &Key<EntityId>,
    place: &Key<EntityId>,
    containers: &BTreeMap<Key<EntityId>, Key<EntityId>>,
) -> bool {
    let mut current = Some(place);
    for _ in 0..=containers.len() {
        match current {
            None => return false,
            Some(node) if node == root => return true,
            Some(node) => current = containers.get(node),
        }
    }
    false
}

/// Whether a place reaches itself through its container chain: the same walk,
/// started one link up so a place is not its own ancestor by definition.
fn contains_itself(
    start: &Key<EntityId>,
    containers: &BTreeMap<Key<EntityId>, Key<EntityId>>,
) -> bool {
    containers
        .get(start)
        .is_some_and(|parent| key_covers_place(start, parent, containers))
}

/// Whether `place` is `root` or is contained in it, over canonical state: one
/// upward walk in the shape of [`containment_terminates`], bounded by the graph
/// size. It is total because `admit_resolved` refuses a non-terminating chain,
/// so the bound is belt-and-braces rather than a repair loop. There is no
/// downward enumeration and no child index: every jurisdictional question this
/// kernel asks is "is this target inside my ground".
pub(super) fn covers_place(
    entities: &BTreeMap<EntityId, EntityRecord>,
    root: EntityId,
    place: EntityId,
) -> bool {
    let mut current = Some(place);
    for _ in 0..=entities.len() {
        match current {
            None => return false,
            Some(node) if node == root => return true,
            Some(node) => current = entities.get(&node).and_then(|record| record.container),
        }
    }
    false
}

/// The canonical twin of the candidate graph's `candidate_targets_overlap`: whether two
/// jurisdictions of one kind cover common ground.
pub(super) fn targets_overlap(
    entities: &BTreeMap<EntityId, EntityRecord>,
    left: AuthorityTarget,
    right: AuthorityTarget,
) -> bool {
    match (left, right) {
        (AuthorityTarget::Subject(one), AuthorityTarget::Subject(other)) => one == other,
        (AuthorityTarget::PlaceSubtree(one), AuthorityTarget::PlaceSubtree(other)) => {
            covers_place(entities, one, other) || covers_place(entities, other, one)
        }
        _ => false,
    }
}

/// Whether a subject holding `grants` may traverse a route with this access
/// rule toward `destination`. The sole statement of the `Restricted` rule,
/// called by `resolve_patch`'s `Relocate` arm, by `apply_operation`'s, and by
/// `Precondition::Reachable`'s edge admission — one rule, three callers, no
/// drift. Openness is a separate claim with its own name, so it is not folded
/// in here.
pub(super) fn route_admits(
    state: &super::WorldState,
    grants: &BTreeSet<AuthorityGrant>,
    access: &AccessKind,
    destination: EntityId,
) -> bool {
    match access {
        AccessKind::Public => true,
        AccessKind::Restricted { requires } => grants.iter().any(|grant| {
            &grant.kind == requires
                && super::covers(state, grant.over, super::Target::Entity(destination))
        }),
    }
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

// ---------------------------------------------------------------------------
// The patch tool catalog.
//
// The vocabulary's owner owns its projection. `PATCH_TOOLS` is one entry per
// admissible authoring shape, and the shape rule is stated once: the tool
// surface is the variants a non-genesis patch may carry, with every
// always-refused choice removed rather than exposed. Nothing else in the tree
// emits a model-facing patch schema.
// ---------------------------------------------------------------------------

/// What a tool call becomes in the draft patch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PatchToolShape {
    /// Emits `Declaration::<variant>` with `fixed` merged into the arguments.
    Declare {
        variant: &'static str,
        fixed: &'static [(&'static str, &'static str)],
    },
    /// Emits `ComponentOp::<variant>`.
    Operate { variant: &'static str },
    /// Ends or annotates the session; produces nothing for the patch.
    Session,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PatchField {
    pub(crate) name: &'static str,
    pub(crate) kind: PatchFieldKind,
}

/// One argument's projection. Bounds live here rather than only in a rejection,
/// so the model is told the bound instead of discovering it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PatchFieldKind {
    Reference(&'static str),
    OptionalReference(&'static str),
    ReferenceSet(&'static str),
    Handle,
    Label,
    Statement,
    Quantity,
    Cost,
    Magnitude,
    Minutes,
    OptionalPeriod,
    Evidence,
    Flag,
    Text(&'static str),
    Name(&'static str),
    NameSet(&'static str),
    Choice(&'static [&'static str]),
    Composite(CompositeShape),
    CompositeList(CompositeShape),
}

/// A closed payload type the vocabulary already owns. One arm per shape in both
/// the schema emitter and the exemplar builder, so a new shape breaks the build
/// in two places rather than shipping a schema with no decoder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompositeShape {
    NewController,
    AccessKind,
    DependencyRef,
    AuthorityTargetRef,
    AuthorityGrantRef,
    ReachRef,
    FactStandingRef,
    AudienceRef,
    AuthoredSource,
    PressureSourceRef,
    CommitmentKey,
    PreconditionRef,
    RoleSpec,
    Precondition,
    EffectSlot,
    OutcomeBand,
}

pub(crate) struct PatchTool {
    /// Equal to the variant's serde tag, except for the two `Entity` splits and
    /// the two session tools.
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) fields: &'static [PatchField],
    pub(crate) shape: PatchToolShape,
}

const fn field(name: &'static str, kind: PatchFieldKind) -> PatchField {
    PatchField { name, kind }
}

const SUBJECT_KINDS: &[&str] = &["person", "institution", "population"];
const COMMITMENT_KINDS: &[&str] = &["routine", "obligation", "goal"];
const CONFIDENCES: &[&str] = &["doubted", "believed", "certain"];

pub(crate) const RECORD_GAP_PATCH_TOOL: &str = "record_gap";
pub(crate) const SUBMIT_PATCH_TOOL: &str = "submit";

pub(crate) const PATCH_TOOLS: &[PatchTool] = &[
    PatchTool {
        name: "declare_subject",
        description: "Declare a new subject: a person, an institution, or a population.",
        fields: &[
            field("handle", PatchFieldKind::Handle),
            field("label", PatchFieldKind::Label),
            field("kind", PatchFieldKind::Choice(SUBJECT_KINDS)),
            field(
                "controller",
                PatchFieldKind::Composite(CompositeShape::NewController),
            ),
            field("affordances", PatchFieldKind::ReferenceSet("affordance")),
            field("position", PatchFieldKind::OptionalReference("place")),
        ],
        shape: PatchToolShape::Declare {
            variant: "subject",
            fixed: &[],
        },
    },
    PatchTool {
        name: "declare_place",
        description: "Declare a place. Its container, when given, must be a place.",
        fields: &[
            field("handle", PatchFieldKind::Handle),
            field("label", PatchFieldKind::Label),
            field("container", PatchFieldKind::OptionalReference("place")),
        ],
        shape: PatchToolShape::Declare {
            variant: "entity",
            fixed: &[("kind", "place")],
        },
    },
    PatchTool {
        name: "declare_resource",
        description: "Declare a resource kind. Quantity is admitted separately, with evidence.",
        fields: &[
            field("handle", PatchFieldKind::Handle),
            field("label", PatchFieldKind::Label),
        ],
        shape: PatchToolShape::Declare {
            variant: "entity",
            fixed: &[("kind", "resource")],
        },
    },
    PatchTool {
        name: "declare_route",
        description: "Declare an open route between two places.",
        fields: &[
            field("handle", PatchFieldKind::Handle),
            field("label", PatchFieldKind::Label),
            field("from", PatchFieldKind::Reference("place")),
            field("to", PatchFieldKind::Reference("place")),
            field(
                "access",
                PatchFieldKind::Composite(CompositeShape::AccessKind),
            ),
            field("cost", PatchFieldKind::Cost),
        ],
        shape: PatchToolShape::Declare {
            variant: "route",
            fixed: &[],
        },
    },
    PatchTool {
        name: "declare_affordance",
        description: "Declare one catalog entry: what an affordance is, not who may use it.",
        fields: &[
            field("handle", PatchFieldKind::Handle),
            field("kind", PatchFieldKind::Name("the affordance's tool name")),
            field(
                "roles",
                PatchFieldKind::CompositeList(CompositeShape::RoleSpec),
            ),
            field(
                "preconditions",
                PatchFieldKind::CompositeList(CompositeShape::Precondition),
            ),
            field(
                "effect_slots",
                PatchFieldKind::CompositeList(CompositeShape::EffectSlot),
            ),
            field(
                "outcome_bands",
                PatchFieldKind::CompositeList(CompositeShape::OutcomeBand),
            ),
            field("carries_speech", PatchFieldKind::Flag),
        ],
        shape: PatchToolShape::Declare {
            variant: "affordance",
            fixed: &[],
        },
    },
    PatchTool {
        name: "declare_fact",
        description: "Declare a fact: a short authored name and the statement it carries.",
        fields: &[
            field("handle", PatchFieldKind::Handle),
            field("label", PatchFieldKind::Label),
            field("statement", PatchFieldKind::Statement),
            field(
                "standing",
                PatchFieldKind::Composite(CompositeShape::FactStandingRef),
            ),
        ],
        shape: PatchToolShape::Declare {
            variant: "fact",
            fixed: &[],
        },
    },
    PatchTool {
        name: "declare_channel",
        description: "Declare a channel and the reach it carries over.",
        fields: &[
            field("handle", PatchFieldKind::Handle),
            field("label", PatchFieldKind::Label),
            field("reach", PatchFieldKind::Composite(CompositeShape::ReachRef)),
            field("controller", PatchFieldKind::OptionalReference("subject")),
        ],
        shape: PatchToolShape::Declare {
            variant: "channel",
            fixed: &[],
        },
    },
    PatchTool {
        name: "relocate",
        description: "Move a subject along one open route it may traverse.",
        fields: &[
            field("subject", PatchFieldKind::Reference("subject")),
            field("via", PatchFieldKind::Reference("route")),
        ],
        shape: PatchToolShape::Operate {
            variant: "relocate",
        },
    },
    PatchTool {
        name: "open_route",
        description: "Open a closed route.",
        fields: &[field("route", PatchFieldKind::Reference("route"))],
        shape: PatchToolShape::Operate {
            variant: "open_route",
        },
    },
    PatchTool {
        name: "close_route",
        description: "Close an open route.",
        fields: &[field("route", PatchFieldKind::Reference("route"))],
        shape: PatchToolShape::Operate {
            variant: "close_route",
        },
    },
    PatchTool {
        name: "alter_cost",
        description: "Set a route's traversal cost in minutes.",
        fields: &[
            field("route", PatchFieldKind::Reference("route")),
            field("cost", PatchFieldKind::Cost),
        ],
        shape: PatchToolShape::Operate {
            variant: "alter_cost",
        },
    },
    PatchTool {
        name: "transfer",
        description: "Move a held quantity of one resource from one subject to another.",
        fields: &[
            field("from", PatchFieldKind::Reference("subject")),
            field("to", PatchFieldKind::Reference("subject")),
            field("resource", PatchFieldKind::Reference("resource")),
            field("qty", PatchFieldKind::Quantity),
        ],
        shape: PatchToolShape::Operate {
            variant: "transfer",
        },
    },
    PatchTool {
        name: "transform",
        description: "Relabel a held quantity one for one: the same quantity leaves and arrives.",
        fields: &[
            field("holder", PatchFieldKind::Reference("subject")),
            field("from_resource", PatchFieldKind::Reference("resource")),
            field("into_resource", PatchFieldKind::Reference("resource")),
            field("qty", PatchFieldKind::Quantity),
        ],
        shape: PatchToolShape::Operate {
            variant: "transform",
        },
    },
    PatchTool {
        name: "consume",
        description: "Destroy a held quantity.",
        fields: &[
            field("holder", PatchFieldKind::Reference("subject")),
            field("resource", PatchFieldKind::Reference("resource")),
            field("qty", PatchFieldKind::Quantity),
        ],
        shape: PatchToolShape::Operate { variant: "consume" },
    },
    PatchTool {
        name: "admit",
        description: "The only creation path for quantity. Its evidence must be cited in this same patch.",
        fields: &[
            field("holder", PatchFieldKind::Reference("subject")),
            field("resource", PatchFieldKind::Reference("resource")),
            field("qty", PatchFieldKind::Quantity),
            field("evidence", PatchFieldKind::Evidence),
        ],
        shape: PatchToolShape::Operate { variant: "admit" },
    },
    PatchTool {
        name: "bind",
        description: "Record that a subject depends on a resource, a route, or another subject.",
        fields: &[
            field("subject", PatchFieldKind::Reference("subject")),
            field(
                "target",
                PatchFieldKind::Composite(CompositeShape::DependencyRef),
            ),
        ],
        shape: PatchToolShape::Operate { variant: "bind" },
    },
    PatchTool {
        name: "release",
        description: "Remove one dependency of a subject.",
        fields: &[
            field("subject", PatchFieldKind::Reference("subject")),
            field(
                "target",
                PatchFieldKind::Composite(CompositeShape::DependencyRef),
            ),
        ],
        shape: PatchToolShape::Operate { variant: "release" },
    },
    PatchTool {
        name: "grant_authority",
        description: "Add one jurisdiction to a subject: a kind, and the ground it runs over.",
        fields: &[
            field("holder", PatchFieldKind::Reference("subject")),
            field(
                "grant",
                PatchFieldKind::Composite(CompositeShape::AuthorityGrantRef),
            ),
        ],
        shape: PatchToolShape::Operate {
            variant: "grant_authority",
        },
    },
    PatchTool {
        name: "revoke_authority",
        description: "Remove one jurisdiction from a subject.",
        fields: &[
            field("holder", PatchFieldKind::Reference("subject")),
            field(
                "grant",
                PatchFieldKind::Composite(CompositeShape::AuthorityGrantRef),
            ),
        ],
        shape: PatchToolShape::Operate {
            variant: "revoke_authority",
        },
    },
    PatchTool {
        name: "open_office",
        description: "Constitute a seat inside an institution and the authority kinds it lends.",
        fields: &[
            field("institution", PatchFieldKind::Reference("subject")),
            field("office", PatchFieldKind::Name("the office's name")),
            field(
                "delegated",
                PatchFieldKind::NameSet("an authority kind the office lends"),
            ),
        ],
        shape: PatchToolShape::Operate {
            variant: "open_office",
        },
    },
    PatchTool {
        name: "close_office",
        description: "Dissolve a seat inside an institution.",
        fields: &[
            field("institution", PatchFieldKind::Reference("subject")),
            field("office", PatchFieldKind::Name("the office's name")),
        ],
        shape: PatchToolShape::Operate {
            variant: "close_office",
        },
    },
    PatchTool {
        name: "install_incumbent",
        description: "Seat a person in one of an institution's offices.",
        fields: &[
            field("institution", PatchFieldKind::Reference("subject")),
            field("office", PatchFieldKind::Name("the office's name")),
            field("incumbent", PatchFieldKind::Reference("subject")),
        ],
        shape: PatchToolShape::Operate {
            variant: "install_incumbent",
        },
    },
    PatchTool {
        name: "vacate_office",
        description: "Empty a seat, preserving the office itself.",
        fields: &[
            field("institution", PatchFieldKind::Reference("subject")),
            field("office", PatchFieldKind::Name("the office's name")),
        ],
        shape: PatchToolShape::Operate {
            variant: "vacate_office",
        },
    },
    PatchTool {
        name: "open_forum",
        description: "Say where one kind of grievance goes, and who may bring it.",
        fields: &[
            field("grievance", PatchFieldKind::Name("the grievance kind")),
            field("forum", PatchFieldKind::Reference("subject")),
            field(
                "standing",
                PatchFieldKind::Composite(CompositeShape::AuthorityTargetRef),
            ),
        ],
        shape: PatchToolShape::Operate {
            variant: "open_forum",
        },
    },
    PatchTool {
        name: "close_forum",
        description: "Remove the forum that takes one kind of grievance.",
        fields: &[field(
            "grievance",
            PatchFieldKind::Name("the grievance kind"),
        )],
        shape: PatchToolShape::Operate {
            variant: "close_forum",
        },
    },
    PatchTool {
        name: "acquire_knowledge",
        description: "Give a subject a fact it witnessed, or that a canonical receipt evidences.",
        fields: &[
            field("subject", PatchFieldKind::Reference("subject")),
            field("fact", PatchFieldKind::Reference("fact")),
            field(
                "source",
                PatchFieldKind::Composite(CompositeShape::AuthoredSource),
            ),
            field("confidence", PatchFieldKind::Choice(CONFIDENCES)),
        ],
        shape: PatchToolShape::Operate {
            variant: "acquire_knowledge",
        },
    },
    PatchTool {
        name: "communicate",
        description: "One telling. The recipients are re-derived from live positions and channels.",
        fields: &[
            field("speaker", PatchFieldKind::Reference("subject")),
            field("fact", PatchFieldKind::Reference("fact")),
            field("to", PatchFieldKind::Composite(CompositeShape::AudienceRef)),
        ],
        shape: PatchToolShape::Operate {
            variant: "communicate",
        },
    },
    PatchTool {
        name: "forget",
        description: "Remove one fact from one subject's knowledge.",
        fields: &[
            field("subject", PatchFieldKind::Reference("subject")),
            field("fact", PatchFieldKind::Reference("fact")),
        ],
        shape: PatchToolShape::Operate { variant: "forget" },
    },
    PatchTool {
        name: "set_reach",
        description: "Set which subjects a channel carries to.",
        fields: &[
            field("channel", PatchFieldKind::Reference("channel")),
            field("reach", PatchFieldKind::Composite(CompositeShape::ReachRef)),
        ],
        shape: PatchToolShape::Operate {
            variant: "set_reach",
        },
    },
    PatchTool {
        name: "set_controller",
        description: "Set, or clear, the subject that may speak on a channel from outside its reach.",
        fields: &[
            field("channel", PatchFieldKind::Reference("channel")),
            field("controller", PatchFieldKind::OptionalReference("subject")),
        ],
        shape: PatchToolShape::Operate {
            variant: "set_controller",
        },
    },
    PatchTool {
        name: "create_commitment",
        description: "Author a promise: a routine with a period, an obligation to another, or a personal goal.",
        fields: &[
            field("subject", PatchFieldKind::Reference("subject")),
            field("counterparty", PatchFieldKind::OptionalReference("subject")),
            field("kind", PatchFieldKind::Choice(COMMITMENT_KINDS)),
            field("due", PatchFieldKind::Minutes),
            field("period", PatchFieldKind::OptionalPeriod),
            field(
                "checks",
                PatchFieldKind::CompositeList(CompositeShape::PreconditionRef),
            ),
        ],
        shape: PatchToolShape::Operate {
            variant: "create_commitment",
        },
    },
    PatchTool {
        name: "discharge_commitment",
        description: "Remove a commitment and every pressure row it sources.",
        fields: &[
            field("subject", PatchFieldKind::Reference("subject")),
            field(
                "key",
                PatchFieldKind::Composite(CompositeShape::CommitmentKey),
            ),
        ],
        shape: PatchToolShape::Operate {
            variant: "discharge_commitment",
        },
    },
    PatchTool {
        name: "advance_pressure",
        description: "Add pressure on a subject from one named source.",
        fields: &[
            field(
                "source",
                PatchFieldKind::Composite(CompositeShape::PressureSourceRef),
            ),
            field("target", PatchFieldKind::Reference("subject")),
            field("by", PatchFieldKind::Magnitude),
        ],
        shape: PatchToolShape::Operate {
            variant: "advance_pressure",
        },
    },
    PatchTool {
        name: "reduce_pressure",
        description: "Subtract pressure on a subject from one named source.",
        fields: &[
            field(
                "source",
                PatchFieldKind::Composite(CompositeShape::PressureSourceRef),
            ),
            field("target", PatchFieldKind::Reference("subject")),
            field("by", PatchFieldKind::Magnitude),
        ],
        shape: PatchToolShape::Operate {
            variant: "reduce_pressure",
        },
    },
    PatchTool {
        name: "resolve_pressure",
        description: "Remove one pressure row entirely.",
        fields: &[
            field(
                "source",
                PatchFieldKind::Composite(CompositeShape::PressureSourceRef),
            ),
            field("target", PatchFieldKind::Reference("subject")),
        ],
        shape: PatchToolShape::Operate {
            variant: "resolve_pressure",
        },
    },
    PatchTool {
        name: RECORD_GAP_PATCH_TOOL,
        description: "Record something the world needs that this vocabulary cannot say. It changes nothing.",
        fields: &[field(
            "detail",
            PatchFieldKind::Text("what the world needs and this vocabulary cannot express"),
        )],
        shape: PatchToolShape::Session,
    },
    PatchTool {
        name: SUBMIT_PATCH_TOOL,
        description: "Submit the accumulated patch for admission. Terminal.",
        fields: &[],
        shape: PatchToolShape::Session,
    },
];

/// The catalog. Fixed per build: it reads `PATCH_TOOLS` and nothing else — not
/// world state, not a snapshot, not a config.
pub(crate) fn patch_tools() -> Vec<CodexToolDefinition> {
    PATCH_TOOLS
        .iter()
        .map(|entry| {
            tool_schema::tool(
                entry.name,
                entry.description,
                tool_schema::object(
                    entry
                        .fields
                        .iter()
                        .map(|field| (field.name.to_owned(), field_schema(field.kind)))
                        .collect(),
                ),
            )
        })
        .collect()
}

/// The same iteration rendered as one prose line, so the prompt's tool list and
/// the schemas have one owner and cannot drift.
pub(crate) fn patch_tool_signatures() -> String {
    PATCH_TOOLS
        .iter()
        .map(|entry| {
            let parameters: Vec<&str> = entry.fields.iter().map(|field| field.name).collect();
            format!("{}({})", entry.name, parameters.join(", "))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn field_schema(kind: PatchFieldKind) -> Value {
    match kind {
        PatchFieldKind::Reference(referent) => tool_schema::reference(referent),
        PatchFieldKind::OptionalReference(referent) => {
            tool_schema::nullable(tool_schema::reference(referent))
        }
        PatchFieldKind::ReferenceSet(referent) => {
            tool_schema::list(tool_schema::reference(referent))
        }
        PatchFieldKind::Handle => {
            tool_schema::canonical_string("a new draft handle, unique within this patch")
        }
        PatchFieldKind::Label => tool_schema::canonical_string("a display label"),
        PatchFieldKind::Statement => {
            tool_schema::canonical_string("the committed world text this fact carries")
        }
        PatchFieldKind::Quantity => tool_schema::bounded_integer(1, u64::MAX),
        PatchFieldKind::Cost => tool_schema::bounded_integer(1, u64::from(MAX_ROUTE_COST)),
        PatchFieldKind::Magnitude => tool_schema::bounded_integer(1, u64::from(u32::MAX)),
        PatchFieldKind::Minutes => tool_schema::bounded_integer(0, u64::MAX),
        PatchFieldKind::OptionalPeriod => {
            tool_schema::nullable(tool_schema::bounded_integer(1, u64::from(MAX_ROUTE_COST)))
        }
        PatchFieldKind::Evidence => {
            tool_schema::canonical_string("an exact evidence receipt retrieved for this answer")
        }
        PatchFieldKind::Flag => json!({"type": "boolean"}),
        PatchFieldKind::Text(description) | PatchFieldKind::Name(description) => {
            tool_schema::canonical_string(description)
        }
        PatchFieldKind::NameSet(description) => {
            tool_schema::list(tool_schema::canonical_string(description))
        }
        PatchFieldKind::Choice(values) => tool_schema::name_enum(values),
        PatchFieldKind::Composite(shape) => composite_schema(shape),
        PatchFieldKind::CompositeList(shape) => tool_schema::list(composite_schema(shape)),
    }
}

fn subject_ref_schema() -> Value {
    tool_schema::reference("subject")
}

fn composite_schema(shape: CompositeShape) -> Value {
    match shape {
        // `human` is absent, not present-and-rejected: `UnadmittedController`
        // refuses it outside genesis, so exposing it would be a door slammed.
        CompositeShape::NewController => tool_schema::variant(
            "type",
            vec![("narrative_persona", vec![]), ("operational_agent", vec![])],
        ),
        CompositeShape::AccessKind => tool_schema::variant(
            "access",
            vec![
                ("public", vec![]),
                (
                    "restricted",
                    vec![(
                        "requires".to_owned(),
                        tool_schema::canonical_string("the authority kind that opens this route"),
                    )],
                ),
            ],
        ),
        CompositeShape::DependencyRef => tool_schema::variant_content(
            "target",
            "ref",
            vec![
                ("resource", Some(tool_schema::reference("resource"))),
                ("route", Some(tool_schema::reference("route"))),
                ("subject", Some(subject_ref_schema())),
            ],
        ),
        CompositeShape::AuthorityTargetRef => tool_schema::variant_content(
            "over",
            "ref",
            vec![
                ("subject", Some(subject_ref_schema())),
                ("place_subtree", Some(tool_schema::reference("place"))),
            ],
        ),
        CompositeShape::AuthorityGrantRef => tool_schema::object(vec![
            (
                "kind".to_owned(),
                tool_schema::canonical_string("the authority kind"),
            ),
            (
                "over".to_owned(),
                composite_schema(CompositeShape::AuthorityTargetRef),
            ),
        ]),
        CompositeShape::ReachRef => tool_schema::variant_content(
            "reach",
            "of",
            vec![
                ("subjects", Some(tool_schema::list(subject_ref_schema()))),
                ("place", Some(tool_schema::reference("place"))),
            ],
        ),
        CompositeShape::FactStandingRef => tool_schema::variant(
            "standing",
            vec![
                (
                    "canonical",
                    vec![(
                        "evidence".to_owned(),
                        tool_schema::canonical_string("an exact evidence receipt"),
                    )],
                ),
                ("claimed", vec![("by".to_owned(), subject_ref_schema())]),
            ],
        ),
        CompositeShape::AudienceRef => tool_schema::external_variant(vec![
            ("colocated", None),
            ("channel", Some(tool_schema::reference("channel"))),
        ]),
        CompositeShape::AuthoredSource => {
            tool_schema::variant("source", vec![("witnessed", vec![]), ("evidenced", vec![])])
        }
        CompositeShape::PressureSourceRef => tool_schema::variant_content(
            "from",
            "of",
            vec![
                (
                    "commitment",
                    Some(tool_schema::object(vec![
                        ("subject".to_owned(), subject_ref_schema()),
                        (
                            "key".to_owned(),
                            composite_schema(CompositeShape::CommitmentKey),
                        ),
                    ])),
                ),
                (
                    "dependency",
                    Some(composite_schema(CompositeShape::DependencyRef)),
                ),
                ("subject", Some(subject_ref_schema())),
            ],
        ),
        CompositeShape::CommitmentKey => tool_schema::object(vec![
            (
                "command".to_owned(),
                tool_schema::canonical_string("the command id that created the commitment"),
            ),
            (
                "index".to_owned(),
                tool_schema::bounded_integer(0, u64::from(u32::MAX)),
            ),
        ]),
        CompositeShape::PreconditionRef => precondition_ref_schema(),
        CompositeShape::RoleSpec => tool_schema::object(vec![
            (
                "role".to_owned(),
                tool_schema::canonical_string("the slot's parameter name"),
            ),
            ("kind".to_owned(), ref_kind_schema()),
        ]),
        CompositeShape::Precondition => precondition_schema(),
        CompositeShape::EffectSlot => tool_schema::object(vec![
            ("op_kind".to_owned(), component_op_kind_schema()),
            (
                "roles".to_owned(),
                tool_schema::list(tool_schema::canonical_string("a declared role name")),
            ),
            (
                "bounds".to_owned(),
                tool_schema::variant_content(
                    "bound",
                    "max",
                    vec![
                        ("none", None),
                        ("quantity", Some(tool_schema::bounded_integer(1, u64::MAX))),
                        (
                            "cost",
                            Some(tool_schema::bounded_integer(1, u64::from(MAX_ROUTE_COST))),
                        ),
                    ],
                ),
            ),
        ]),
        CompositeShape::OutcomeBand => tool_schema::object(vec![
            (
                "weight".to_owned(),
                tool_schema::bounded_integer(1, u64::from(u32::MAX)),
            ),
            (
                "effects".to_owned(),
                tool_schema::list(tool_schema::bounded_integer(0, u64::from(u32::MAX))),
            ),
        ]),
    }
}

fn ref_kind_schema() -> Value {
    tool_schema::variant_content(
        "namespace",
        "kind",
        vec![
            (
                "subject",
                Some(tool_schema::nullable(tool_schema::name_enum(SUBJECT_KINDS))),
            ),
            (
                "entity",
                Some(tool_schema::name_enum(&[
                    "place", "resource", "fact", "channel",
                ])),
            ),
            ("edge", Some(tool_schema::name_enum(&["route"]))),
        ],
    )
}

/// The referent-naming checks a commitment carries.
fn precondition_ref_schema() -> Value {
    tool_schema::variant(
        "precondition",
        vec![
            (
                "present",
                vec![("at".to_owned(), tool_schema::reference("place"))],
            ),
            (
                "reachable",
                vec![
                    ("to".to_owned(), tool_schema::reference("place")),
                    (
                        "within".to_owned(),
                        tool_schema::bounded_integer(1, u64::from(MAX_ROUTE_COST)),
                    ),
                ],
            ),
            (
                "holds",
                vec![
                    ("resource".to_owned(), tool_schema::reference("resource")),
                    (
                        "at_least".to_owned(),
                        tool_schema::bounded_integer(1, u64::MAX),
                    ),
                ],
            ),
            (
                "authorized",
                vec![
                    (
                        "over".to_owned(),
                        composite_schema(CompositeShape::AuthorityTargetRef),
                    ),
                    (
                        "kind".to_owned(),
                        tool_schema::canonical_string("the authority kind"),
                    ),
                ],
            ),
            (
                "has_standing",
                vec![(
                    "grievance".to_owned(),
                    tool_schema::canonical_string("the grievance kind"),
                )],
            ),
            (
                "knows",
                vec![
                    ("fact".to_owned(), tool_schema::reference("fact")),
                    ("at_least".to_owned(), tool_schema::name_enum(CONFIDENCES)),
                ],
            ),
            (
                "can_broadcast",
                vec![(
                    "via".to_owned(),
                    composite_schema(CompositeShape::AudienceRef),
                )],
            ),
            (
                "can_reach",
                vec![
                    ("subject".to_owned(), subject_ref_schema()),
                    (
                        "via".to_owned(),
                        composite_schema(CompositeShape::AudienceRef),
                    ),
                ],
            ),
            (
                "committed",
                vec![
                    ("to".to_owned(), subject_ref_schema()),
                    ("kind".to_owned(), tool_schema::name_enum(COMMITMENT_KINDS)),
                ],
            ),
        ],
    )
}

/// The role-naming twin. A catalog entry's checks name roles the entry
/// declares, never referents.
fn precondition_schema() -> Value {
    let role = || tool_schema::canonical_string("a declared role name");
    let audience_spec =
        || tool_schema::external_variant(vec![("colocated", None), ("channel", Some(role()))]);
    tool_schema::variant(
        "precondition",
        vec![
            ("present", vec![("at".to_owned(), role())]),
            (
                "reachable",
                vec![
                    ("to".to_owned(), role()),
                    (
                        "within".to_owned(),
                        tool_schema::bounded_integer(1, u64::from(MAX_ROUTE_COST)),
                    ),
                ],
            ),
            (
                "holds",
                vec![
                    ("resource".to_owned(), role()),
                    (
                        "at_least".to_owned(),
                        tool_schema::bounded_integer(1, u64::MAX),
                    ),
                ],
            ),
            (
                "authorized",
                vec![
                    ("over".to_owned(), role()),
                    (
                        "kind".to_owned(),
                        tool_schema::canonical_string("the authority kind"),
                    ),
                ],
            ),
            (
                "has_standing",
                vec![(
                    "grievance".to_owned(),
                    tool_schema::canonical_string("the grievance kind"),
                )],
            ),
            (
                "knows",
                vec![
                    ("fact".to_owned(), role()),
                    ("at_least".to_owned(), tool_schema::name_enum(CONFIDENCES)),
                ],
            ),
            ("can_broadcast", vec![("via".to_owned(), audience_spec())]),
            (
                "can_reach",
                vec![
                    ("subject".to_owned(), role()),
                    ("via".to_owned(), audience_spec()),
                ],
            ),
            (
                "committed",
                vec![
                    ("to".to_owned(), role()),
                    ("kind".to_owned(), tool_schema::name_enum(COMMITMENT_KINDS)),
                ],
            ),
        ],
    )
}

/// Exactly the operations an affordance may propose, with the payload the world
/// fixes when it authors the entry.
fn component_op_kind_schema() -> Value {
    let authority_kind = || {
        vec![(
            "kind".to_owned(),
            tool_schema::canonical_string("the authority kind"),
        )]
    };
    let office = || {
        vec![(
            "office".to_owned(),
            tool_schema::canonical_string("the office's name"),
        )]
    };
    tool_schema::variant(
        "op",
        vec![
            ("relocate", vec![]),
            ("open_route", vec![]),
            ("close_route", vec![]),
            ("alter_cost", vec![]),
            ("transfer", vec![]),
            ("transform", vec![]),
            ("consume", vec![]),
            ("bind", vec![]),
            ("release", vec![]),
            ("grant_authority", authority_kind()),
            ("revoke_authority", authority_kind()),
            ("install_incumbent", office()),
            ("vacate_office", office()),
            (
                "acquire_knowledge",
                vec![("confidence".to_owned(), tool_schema::name_enum(CONFIDENCES))],
            ),
            ("forget", vec![]),
            (
                "create_commitment",
                vec![
                    ("kind".to_owned(), tool_schema::name_enum(COMMITMENT_KINDS)),
                    (
                        "horizon".to_owned(),
                        tool_schema::bounded_integer(1, u64::from(MAX_ROUTE_COST)),
                    ),
                    (
                        "period".to_owned(),
                        tool_schema::nullable(tool_schema::bounded_integer(
                            1,
                            u64::from(MAX_ROUTE_COST),
                        )),
                    ),
                ],
            ),
            (
                "advance_pressure",
                vec![(
                    "by".to_owned(),
                    tool_schema::bounded_integer(1, u64::from(u32::MAX)),
                )],
            ),
            (
                "reduce_pressure",
                vec![(
                    "by".to_owned(),
                    tool_schema::bounded_integer(1, u64::from(u32::MAX)),
                )],
            ),
            ("resolve_pressure", vec![]),
        ],
    )
}

/// One example value per field kind, used by the round-trip test to prove the
/// const list's field names and shapes against the real structs. It is not a
/// second schema: serde is the only decoder, so a wrong name cannot survive.
#[cfg(test)]
pub(super) fn field_example(kind: PatchFieldKind) -> Value {
    let draft = |handle: &str| json!({"ref": "draft", "value": handle});
    match kind {
        PatchFieldKind::Reference(referent) => draft(referent),
        PatchFieldKind::OptionalReference(_) => Value::Null,
        PatchFieldKind::ReferenceSet(referent) => json!([draft(referent)]),
        PatchFieldKind::Handle => json!("example_handle"),
        PatchFieldKind::Label => json!("Example Label"),
        PatchFieldKind::Statement => json!("The lower hinge flooded."),
        PatchFieldKind::Quantity | PatchFieldKind::Minutes => json!(7),
        PatchFieldKind::Cost | PatchFieldKind::Magnitude => json!(3),
        PatchFieldKind::OptionalPeriod => Value::Null,
        PatchFieldKind::Evidence => json!("vault:example/1"),
        PatchFieldKind::Flag => json!(false),
        PatchFieldKind::Text(_) | PatchFieldKind::Name(_) => json!("example_name"),
        PatchFieldKind::NameSet(_) => json!(["example_name"]),
        PatchFieldKind::Choice(values) => json!(values[0]),
        PatchFieldKind::Composite(shape) => composite_example(shape),
        PatchFieldKind::CompositeList(shape) => json!([composite_example(shape)]),
    }
}

/// A fixed canonical string that also decodes as a canonical id: valid as any
/// plain string field and, spelled as the nil UUID, valid wherever a field is
/// actually typed as a command or entity id.
#[cfg(test)]
const ANY_STRING: &str = "00000000-0000-0000-0000-000000000000";

#[cfg(test)]
fn composite_example(shape: CompositeShape) -> Value {
    schema_derived_example(&composite_schema(shape))
}

/// The one derivation of an example value from an emitted schema fragment:
/// the first branch of every sum, the minimum of every bounded integer, a
/// fixed canonical string, and an empty list wherever the schema allows one.
/// `composite_example` is a projection of this over `composite_schema`
/// rather than a hand-written third spelling of the same vocabulary.
#[cfg(test)]
fn schema_derived_example(schema: &Value) -> Value {
    if let Some(alternatives) = schema.get("oneOf").and_then(Value::as_array) {
        return schema_derived_example(alternatives.first().unwrap_or(&Value::Null));
    }
    if let Some(constant) = schema.get("const") {
        return constant.clone();
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        return values.first().cloned().unwrap_or(Value::Null);
    }
    match schema.get("type").and_then(Value::as_str) {
        Some("object") => Value::Object(
            schema
                .get("properties")
                .and_then(Value::as_object)
                .map(|properties| {
                    properties
                        .iter()
                        .map(|(name, property)| (name.clone(), schema_derived_example(property)))
                        .collect()
                })
                .unwrap_or_default(),
        ),
        Some("array") => Value::Array(Vec::new()),
        Some("integer") => schema.get("minimum").cloned().unwrap_or_else(|| json!(0)),
        Some("boolean") => json!(false),
        Some("string") => json!(ANY_STRING),
        _ => Value::Null,
    }
}

/// Verification 16: the model-facing patch catalog is exactly the vocabulary
/// the reducer owns, with every always-refused choice removed rather than
/// exposed.
#[cfg(test)]
mod catalog_tests {
    use super::*;

    /// One arguments object per entry, one example value per field, with the
    /// entry's serde tag and its fixed pairs injected exactly as the evaluator
    /// injects them.
    fn arguments_for(entry: &PatchTool) -> Value {
        let mut object = serde_json::Map::new();
        for field in entry.fields {
            object.insert(field.name.to_owned(), field_example(field.kind));
        }
        match entry.shape {
            PatchToolShape::Declare { variant, fixed } => {
                object.insert("type".into(), Value::String(variant.into()));
                for (key, value) in fixed {
                    object.insert((*key).into(), Value::String((*value).into()));
                }
            }
            PatchToolShape::Operate { variant } => {
                object.insert("op".into(), Value::String(variant.into()));
            }
            PatchToolShape::Session => {}
        }
        Value::Object(object)
    }

    fn tag_of<T: Serialize>(value: &T, tag: &str) -> String {
        serde_json::to_value(value).expect("the vocabulary serializes")[tag]
            .as_str()
            .expect("an internally tagged variant")
            .to_owned()
    }

    /// Serde is the only decoder, so a field name the const list gets wrong
    /// cannot survive this. It pins the catalog's argument names and shapes to
    /// the real struct fields, in both directions. Each field's example is
    /// itself a projection of the emitted schema (`schema_derived_example`),
    /// so checking it against `field_schema` here proves the schema-derived
    /// example against serde in one place, for every tool.
    #[test]
    fn patch_tool_arguments_round_trip_into_the_vocabulary() {
        for entry in PATCH_TOOLS {
            for field in entry.fields {
                let example = field_example(field.kind);
                assert!(
                    admits(&field_schema(field.kind), &example),
                    "{} field {}: the schema-derived example does not satisfy its own schema: {example}",
                    entry.name,
                    field.name
                );
            }
            let arguments = arguments_for(entry);
            match entry.shape {
                PatchToolShape::Declare { variant, fixed } => {
                    let declaration: Declaration = serde_json::from_value(arguments.clone())
                        .unwrap_or_else(|error| {
                            panic!("{} did not decode: {error} from {arguments}", entry.name)
                        });
                    assert_eq!(tag_of(&declaration, "type"), variant, "{}", entry.name);
                    let round_tripped =
                        serde_json::to_value(&declaration).expect("a declaration re-encodes");
                    for (key, value) in fixed {
                        assert_eq!(round_tripped[*key], Value::String((*value).into()));
                    }
                }
                PatchToolShape::Operate { variant } => {
                    let operation: ComponentOp = serde_json::from_value(arguments.clone())
                        .unwrap_or_else(|error| {
                            panic!("{} did not decode: {error} from {arguments}", entry.name)
                        });
                    assert_eq!(tag_of(&operation, "op"), variant, "{}", entry.name);
                }
                PatchToolShape::Session => {
                    assert!(matches!(arguments, Value::Object(_)));
                }
            }
        }
    }

    /// Enumeration equality. The tool names, the shapes' variant tags expanded
    /// by their fixed choices, and the two session tools are one set.
    #[test]
    fn patch_tool_catalog_equals_the_vocabulary() {
        let emitted: BTreeSet<String> = patch_tools()
            .into_iter()
            .map(|definition| definition.name)
            .collect();
        let listed: BTreeSet<String> = PATCH_TOOLS
            .iter()
            .map(|entry| entry.name.to_owned())
            .collect();
        assert_eq!(emitted, listed, "the generator invented or dropped a tool");
        assert!(listed.contains(RECORD_GAP_PATCH_TOOL));
        assert!(listed.contains(SUBMIT_PATCH_TOOL));

        // Seven declaration shapes over six `Declaration` variants — `Entity`
        // splits by kind, because a `kind` field would be a door slammed for
        // half its values — plus twenty-eight operations and two session tools.
        // `assert_exhaustive` below breaks the build when a variant is added;
        // these counts are where the addition is restated.
        let declarations = PATCH_TOOLS
            .iter()
            .filter(|entry| matches!(entry.shape, PatchToolShape::Declare { .. }))
            .count();
        let operations = PATCH_TOOLS
            .iter()
            .filter(|entry| matches!(entry.shape, PatchToolShape::Operate { .. }))
            .count();
        assert_eq!((declarations, operations, PATCH_TOOLS.len()), (7, 28, 37));

        // Every declaration variant the vocabulary owns is reachable, and the
        // two payload-carrying entity kinds are not exposed as an `Entity`
        // choice.
        let declared: BTreeSet<String> = PATCH_TOOLS
            .iter()
            .filter_map(|entry| match entry.shape {
                PatchToolShape::Declare { variant, .. } => Some(variant.to_owned()),
                _ => None,
            })
            .collect();
        assert_eq!(
            declared,
            BTreeSet::from([
                "subject".to_owned(),
                "entity".to_owned(),
                "route".to_owned(),
                "affordance".to_owned(),
                "fact".to_owned(),
                "channel".to_owned(),
            ])
        );
    }

    /// One `Declaration` per declare-shaped `PATCH_TOOLS` entry, decoded the
    /// same way the round-trip test decodes it. The nested match is the
    /// forcing function: it is exhaustive over `Declaration`'s variants, so a
    /// new variant fails to compile here before it can reach `PATCH_TOOLS`
    /// unlabeled. `Entity` names two tools because it is the one variant the
    /// catalog splits by fixed `kind`; every other variant names exactly one.
    fn every_declaration() -> Vec<Declaration> {
        fn tool_names(declaration: &Declaration) -> &'static [&'static str] {
            match declaration {
                Declaration::Subject(_) => &["declare_subject"],
                Declaration::Entity(_) => &["declare_place", "declare_resource"],
                Declaration::Route(_) => &["declare_route"],
                Declaration::Affordance(_) => &["declare_affordance"],
                Declaration::Fact(_) => &["declare_fact"],
                Declaration::Channel(_) => &["declare_channel"],
            }
        }
        PATCH_TOOLS
            .iter()
            .filter(|entry| matches!(entry.shape, PatchToolShape::Declare { .. }))
            .map(|entry| {
                let declaration: Declaration =
                    serde_json::from_value(arguments_for(entry)).expect("the entry decodes");
                assert!(
                    tool_names(&declaration).contains(&entry.name),
                    "{} decoded a declaration its own exhaustive match does not name",
                    entry.name
                );
                declaration
            })
            .collect()
    }

    /// The `ComponentOp` sibling of `every_declaration`: one instance per
    /// operate-shaped `PATCH_TOOLS` entry, with the same exhaustive-match
    /// forcing function. Every `ComponentOp` variant names exactly one tool.
    fn every_component_op() -> Vec<ComponentOp> {
        fn tool_name(operation: &ComponentOp) -> &'static str {
            match operation {
                ComponentOp::Relocate { .. } => "relocate",
                ComponentOp::OpenRoute { .. } => "open_route",
                ComponentOp::CloseRoute { .. } => "close_route",
                ComponentOp::AlterCost { .. } => "alter_cost",
                ComponentOp::Transfer { .. } => "transfer",
                ComponentOp::Transform { .. } => "transform",
                ComponentOp::Consume { .. } => "consume",
                ComponentOp::Admit { .. } => "admit",
                ComponentOp::Bind { .. } => "bind",
                ComponentOp::Release { .. } => "release",
                ComponentOp::GrantAuthority { .. } => "grant_authority",
                ComponentOp::RevokeAuthority { .. } => "revoke_authority",
                ComponentOp::OpenOffice { .. } => "open_office",
                ComponentOp::CloseOffice { .. } => "close_office",
                ComponentOp::InstallIncumbent { .. } => "install_incumbent",
                ComponentOp::VacateOffice { .. } => "vacate_office",
                ComponentOp::OpenForum { .. } => "open_forum",
                ComponentOp::CloseForum { .. } => "close_forum",
                ComponentOp::AcquireKnowledge { .. } => "acquire_knowledge",
                ComponentOp::Communicate { .. } => "communicate",
                ComponentOp::Forget { .. } => "forget",
                ComponentOp::SetReach { .. } => "set_reach",
                ComponentOp::SetController { .. } => "set_controller",
                ComponentOp::CreateCommitment { .. } => "create_commitment",
                ComponentOp::DischargeCommitment { .. } => "discharge_commitment",
                ComponentOp::AdvancePressure { .. } => "advance_pressure",
                ComponentOp::ReducePressure { .. } => "reduce_pressure",
                ComponentOp::ResolvePressure { .. } => "resolve_pressure",
            }
        }
        PATCH_TOOLS
            .iter()
            .filter(|entry| matches!(entry.shape, PatchToolShape::Operate { .. }))
            .map(|entry| {
                let operation: ComponentOp =
                    serde_json::from_value(arguments_for(entry)).expect("the entry decodes");
                assert_eq!(tool_name(&operation), entry.name);
                operation
            })
            .collect()
    }

    /// Whether `entry`'s catalog shape names the encoded value: its serde tag
    /// matches `entry`'s variant and, for a `Declare` entry, every one of its
    /// fixed pairs matches too. This is the acceptance rule
    /// `every_vocabulary_variant_maps_to_exactly_one_patch_tool` uses to prove
    /// each exemplar belongs to exactly one tool.
    fn names(entry: &PatchTool, value: &Value) -> bool {
        match entry.shape {
            PatchToolShape::Declare { variant, fixed } => {
                value.get("type").and_then(Value::as_str) == Some(variant)
                    && fixed.iter().all(|(key, expected)| {
                        value.get(*key).and_then(Value::as_str) == Some(*expected)
                    })
            }
            PatchToolShape::Operate { variant } => {
                value.get("op").and_then(Value::as_str) == Some(variant)
            }
            PatchToolShape::Session => false,
        }
    }

    /// The forcing function: every declaration and operation the vocabulary
    /// can construct is accepted by exactly one `PATCH_TOOLS` entry, and the
    /// exemplar counts equal the tool counts. A new variant that maps onto an
    /// existing tool's name, rather than earning its own, fails here even
    /// though it compiles.
    #[test]
    fn every_vocabulary_variant_maps_to_exactly_one_patch_tool() {
        let declare_tools = PATCH_TOOLS
            .iter()
            .filter(|entry| matches!(entry.shape, PatchToolShape::Declare { .. }))
            .count();
        let operate_tools = PATCH_TOOLS
            .iter()
            .filter(|entry| matches!(entry.shape, PatchToolShape::Operate { .. }))
            .count();
        assert_eq!(
            (declare_tools, operate_tools, PATCH_TOOLS.len()),
            (7, 28, 37)
        );

        let declarations = every_declaration();
        assert_eq!(declarations.len(), declare_tools);
        for declaration in &declarations {
            let value = serde_json::to_value(declaration).expect("a declaration encodes");
            let accepting: Vec<&str> = PATCH_TOOLS
                .iter()
                .filter(|entry| names(entry, &value))
                .map(|entry| entry.name)
                .collect();
            assert_eq!(
                accepting.len(),
                1,
                "{value} is accepted by {accepting:?}, not exactly one tool"
            );
        }

        let operations = every_component_op();
        assert_eq!(operations.len(), operate_tools);
        for operation in &operations {
            let value = serde_json::to_value(operation).expect("an operation encodes");
            let accepting: Vec<&str> = PATCH_TOOLS
                .iter()
                .filter(|entry| names(entry, &value))
                .map(|entry| entry.name)
                .collect();
            assert_eq!(
                accepting.len(),
                1,
                "{value} is accepted by {accepting:?}, not exactly one tool"
            );
        }
    }

    /// The sibling of the action catalog's authority test. A generated patch
    /// schema names structure and nothing about who is asking.
    #[test]
    fn patch_tool_schemas_cannot_claim_authority() {
        for definition in patch_tools() {
            let text = definition.parameters_json.to_lowercase();
            for forbidden in [
                "caller",
                "jurisdiction",
                "command_id",
                "revision",
                "digest",
                "principal",
                "affordance_id",
                "opportunity",
                "world_id",
            ] {
                assert!(
                    !text.contains(forbidden),
                    "{} exposed {forbidden}",
                    definition.name
                );
            }
            // `human` is absent from the controller choice, not
            // present-and-rejected.
            if definition.name == "declare_subject" {
                assert!(
                    !text.contains("human"),
                    "the human controller is a door slammed"
                );
            }
            // The entity split is data, not a model choice: neither entity
            // declaration exposes a `kind` at all, so `fact` and `channel` are
            // absent rather than present-and-rejected. A role binding may still
            // name a fact or a channel, which is a different question.
            if matches!(
                definition.name.as_str(),
                "declare_place" | "declare_resource"
            ) {
                assert!(
                    !text.contains("\"kind\""),
                    "{} exposed an entity kind choice",
                    definition.name
                );
            }
        }
    }

    /// One name, two schemas is the drift the catalog exists to prevent: the
    /// elaborator's gap tool and the interpreter's do not share a constant, and
    /// neither schema deserializes the other's arguments.
    #[test]
    fn elaborator_record_gap_is_not_the_interpreter_record_gap() {
        let ours = patch_tools()
            .into_iter()
            .find(|definition| definition.name == RECORD_GAP_PATCH_TOOL)
            .expect("the catalog carries a gap tool");
        assert!(ours.parameters_json.contains("detail"));
        assert!(
            !ours.parameters_json.contains("source_start_byte"),
            "the elaborator has no source prose and no byte span"
        );
    }

    // ---- Soul: the emitter against the decoder -------------------------
    //
    // `composite_example` is a projection of `composite_schema` (see
    // `schema_derived_example` above), so `patch_tool_arguments_round_trip_into_the_vocabulary`
    // checks the schema-derived example against serde directly. The tests
    // below close the complementary direction that matters for a model: an
    // instance built from the emitted schema must decode, for every branch of
    // every sum the schema offers, not only the one branch the example picks.

    /// One instance of `schema`, taking `branch` at the top-level `oneOf` and
    /// the first alternative everywhere below it. Nothing here reads the const
    /// list: the schema is the only input, which is what makes the decode a
    /// statement about the schema.
    fn instance_of(schema: &Value, branch: usize) -> Value {
        if let Some(alternatives) = schema.get("oneOf").and_then(Value::as_array) {
            let index = branch.min(alternatives.len().saturating_sub(1));
            return instance_of(&alternatives[index], 0);
        }
        if let Some(constant) = schema.get("const") {
            return constant.clone();
        }
        if let Some(values) = schema.get("enum").and_then(Value::as_array) {
            return values.first().cloned().unwrap_or(Value::Null);
        }
        match schema.get("type").and_then(Value::as_str) {
            Some("object") => Value::Object(
                schema
                    .get("properties")
                    .and_then(Value::as_object)
                    .map(|properties| {
                        properties
                            .iter()
                            .map(|(name, property)| (name.clone(), instance_of(property, 0)))
                            .collect()
                    })
                    .unwrap_or_default(),
            ),
            Some("array") => Value::Array(vec![instance_of(
                schema.get("items").unwrap_or(&Value::Null),
                0,
            )]),
            Some("integer") => schema.get("minimum").cloned().unwrap_or_else(|| json!(1)),
            Some("boolean") => json!(false),
            Some("string") => json!(ANY_STRING),
            _ => Value::Null,
        }
    }

    fn branch_count(schema: &Value) -> usize {
        schema
            .get("oneOf")
            .and_then(Value::as_array)
            .map_or(1, Vec::len)
    }

    /// A structural reader of exactly the shapes `tool_schema` emits: `oneOf`,
    /// `const`, `enum`, closed objects with every property required, arrays,
    /// bounded integers, strings, booleans, null.
    fn admits(schema: &Value, value: &Value) -> bool {
        if let Some(alternatives) = schema.get("oneOf").and_then(Value::as_array) {
            return alternatives
                .iter()
                .any(|alternative| admits(alternative, value));
        }
        if let Some(constant) = schema.get("const") {
            return constant == value;
        }
        if let Some(values) = schema.get("enum").and_then(Value::as_array) {
            return values.contains(value);
        }
        match schema.get("type").and_then(Value::as_str) {
            Some("object") => {
                let Some(fields) = value.as_object() else {
                    return false;
                };
                let empty = serde_json::Map::new();
                let properties = schema
                    .get("properties")
                    .and_then(Value::as_object)
                    .unwrap_or(&empty);
                if schema.get("additionalProperties") == Some(&json!(false))
                    && fields.keys().any(|name| !properties.contains_key(name))
                {
                    return false;
                }
                if let Some(required) = schema.get("required").and_then(Value::as_array)
                    && required
                        .iter()
                        .any(|name| name.as_str().is_none_or(|name| !fields.contains_key(name)))
                {
                    return false;
                }
                properties.iter().all(|(name, property)| {
                    fields.get(name).is_none_or(|field| admits(property, field))
                })
            }
            Some("array") => value.as_array().is_some_and(|items| {
                let inner = schema.get("items").unwrap_or(&Value::Null);
                items.iter().all(|item| admits(inner, item))
            }),
            Some("integer") => value.as_u64().is_some_and(|number| {
                schema
                    .get("minimum")
                    .and_then(Value::as_u64)
                    .is_none_or(|minimum| number >= minimum)
                    && schema
                        .get("maximum")
                        .and_then(Value::as_u64)
                        .is_none_or(|maximum| number <= maximum)
            }),
            Some("boolean") => value.is_boolean(),
            Some("string") => value.is_string(),
            Some("null") => value.is_null(),
            _ => true,
        }
    }

    fn emitted_schema(name: &str) -> Value {
        let definition = patch_tools()
            .into_iter()
            .find(|definition| definition.name == name)
            .unwrap_or_else(|| panic!("{name} is not emitted"));
        serde_json::from_str(&definition.parameters_json).expect("an emitted schema is JSON")
    }

    fn decodes(
        entry: &PatchTool,
        mut arguments: serde_json::Map<String, Value>,
    ) -> Result<(), String> {
        match entry.shape {
            PatchToolShape::Declare { variant, fixed } => {
                arguments.insert("type".into(), Value::String(variant.into()));
                for (key, value) in fixed {
                    arguments.insert((*key).into(), Value::String((*value).into()));
                }
                serde_json::from_value::<Declaration>(Value::Object(arguments))
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            }
            PatchToolShape::Operate { variant } => {
                arguments.insert("op".into(), Value::String(variant.into()));
                serde_json::from_value::<ComponentOp>(Value::Object(arguments))
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            }
            PatchToolShape::Session => Ok(()),
        }
    }

    /// The direction a model actually travels: it reads the emitted schema and
    /// writes an instance of it. Every branch the schema offers, in every
    /// argument of every tool, must decode into the vocabulary — otherwise the
    /// catalog advertises a shape the reducer cannot read and the model
    /// discovers it through a gap.
    #[test]
    fn soul_every_branch_the_emitted_schema_offers_decodes_into_the_vocabulary() {
        for entry in PATCH_TOOLS {
            let schema = emitted_schema(entry.name);
            let empty = serde_json::Map::new();
            let properties = schema
                .get("properties")
                .and_then(Value::as_object)
                .unwrap_or(&empty)
                .clone();
            let widest = properties.values().map(branch_count).max().unwrap_or(1);
            for branch in 0..widest {
                let arguments: serde_json::Map<String, Value> = properties
                    .iter()
                    .map(|(name, property)| (name.clone(), instance_of(property, branch)))
                    .collect();
                let rendered = Value::Object(arguments.clone());
                assert!(
                    admits(&schema, &rendered),
                    "{}: the instance built from its own schema does not satisfy it: {rendered}",
                    entry.name
                );
                if let Err(error) = decodes(entry, arguments) {
                    panic!(
                        "{} branch {branch}: the emitted schema offers a shape the decoder refuses ({error}): {rendered}",
                        entry.name
                    );
                }
            }
        }
    }

    /// `Bounds::None` is a live variant of the vocabulary — an effect slot with
    /// no ceiling. `variant_content` now spells a payload-less branch the way
    /// its doc comment always promised, so the emitted schema offers `none`
    /// alongside `quantity` and `cost`, and a world author can declare an
    /// unbounded effect slot.
    #[test]
    fn the_catalog_can_declare_an_unbounded_effect_slot() {
        let unbounded = serde_json::to_value(Bounds::None).expect("bounds encode");
        assert_eq!(unbounded, json!({"bound": "none"}));
        let slot = field_schema(PatchFieldKind::CompositeList(CompositeShape::EffectSlot));
        let bounds = slot["items"]["properties"]["bounds"].clone();
        assert!(
            admits(&bounds, &json!({"bound": "quantity", "max": 1})),
            "the bounded spelling is offered"
        );
        assert!(
            admits(&bounds, &unbounded),
            "the unbounded spelling is offered"
        );
        let decoded: Bounds =
            serde_json::from_value(unbounded).expect("the unbounded spelling decodes");
        assert_eq!(decoded, Bounds::None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::tests::{
        FIXTURE_ENTITIES, activate, admit_topology, auth_principal, command, creation, owner,
        player, reject_owner, speak_entry, submit_owner,
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
        assert_eq!(kernel.state.entities.len(), FIXTURE_ENTITIES);
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
        assert_eq!(kernel.state.entities.len(), 3);
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
        assert_eq!(kernel.state.entities.len(), 3);
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
            // Declaring in Active is elaboration, and elaboration answers a
            // boundary. An unanswered one is refused before resolution.
            assert!(matches!(error, KernelError::AnswerRequired));
        }
        assert_eq!(kernel.journal.commit_count(), commits_before);
        assert_eq!(kernel.snapshot().unwrap(), active);
        assert_eq!(kernel.state.entities.len(), FIXTURE_ENTITIES);
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
        assert_eq!(kernel.state.entities.len(), FIXTURE_ENTITIES);
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

        assert_eq!(snapshot.places.len(), 4);
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
        let first = resolve_patch(&kernel.state, command_id, &patch, None).unwrap();
        let second = resolve_patch(&kernel.state, command_id, &patch, None).unwrap();
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
        assert_eq!(reopened.state.entities.len(), FIXTURE_ENTITIES);
        assert!(
            !reopened
                .state
                .subjects
                .contains_key(&SubjectId(would_be[0]))
        );
    }

    /// `discriminator` is length-prefixed like `handle`, so `Some("")` and
    /// `None` write different bytes into the preimage and cannot collide —
    /// and neither can any other pair of discriminators whose bytes would
    /// otherwise concatenate to the same string.
    #[test]
    fn derive_id_does_not_collide_some_empty_discriminator_with_none() {
        let directory = tempfile::tempdir().unwrap();
        let (kernel, _) = WorldKernel::create(
            &directory.path().join("world.cc"),
            creation(CommandId::new(), "DeriveId"),
            &auth_principal(owner()),
        )
        .unwrap();
        let world_id = kernel.state.world_id;
        let command_id = CommandId::new();
        let handle = DraftHandle::new("a-handle");
        assert_ne!(
            derive_id(SUBJECT_NAMESPACE, world_id, command_id, &handle, None),
            derive_id(SUBJECT_NAMESPACE, world_id, command_id, &handle, Some("")),
        );
        // A concatenation-style collision across the handle/discriminator
        // boundary: without a length prefix on the discriminator, "ab" + ""
        // and "a" + "b" would hash identically.
        assert_ne!(
            derive_id(
                SUBJECT_NAMESPACE,
                world_id,
                command_id,
                &DraftHandle::new("ab"),
                Some(""),
            ),
            derive_id(
                SUBJECT_NAMESPACE,
                world_id,
                command_id,
                &DraftHandle::new("a"),
                Some("b"),
            ),
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
        let resolved = resolve_patch(&kernel.state, CommandId::new(), &patch, None).unwrap();
        let honest = super::super::WorldEffect::PatchAdmitted {
            resolved: resolved.clone(),
            answers: None,
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
            &super::super::WorldEffect::PatchAdmitted {
                answers: None,
                resolved: forged,
            },
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

        // And an unanswered declaring effect is refused in Active before any
        // partition is touched.
        let active = activate(&mut kernel);
        assert_eq!(active.revision, before.revision + 3);
        let mut candidate = kernel.state.clone();
        let unanswered = super::super::apply_effect(
            &mut candidate,
            CommandId::issue(),
            &CallerId::Principal(owner()),
            &honest,
        )
        .unwrap_err();
        assert!(matches!(unanswered, KernelError::AnswerRequired));
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
        // Every fixture subject stands somewhere now, so the unplaced case is
        // declared here rather than borrowed from a genesis accident.
        let before = kernel.snapshot().unwrap();
        let speak = speak_entry(&kernel);
        submit_owner(
            &mut kernel,
            &before,
            admit(patch_of(vec![Declaration::Subject(SubjectDeclaration {
                handle: draft("nowhere"),
                label: "The Unplaced".into(),
                kind: SubjectKind::Person,
                controller: NewController::NarrativePersona,
                affordances: BTreeSet::from([speak]),
                position: None,
            })])),
        );
        let active = activate(&mut kernel);
        let commits_before = kernel.journal.commit_count();
        let unplaced = *kernel
            .state
            .subjects
            .iter()
            .find(|(subject_id, _)| !kernel.state.positions.contains_key(*subject_id))
            .expect("the unplaced subject this test declared")
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
                    facts: Vec::new(),
                    channels: Vec::new(),
                    operations: vec![ResolvedOp::Relocate {
                        subject_id: topology.walker,
                        edge_id,
                    }],
                    evidence: Vec::new(),
                    scale_intent: None,
                },
                answers: None,
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
