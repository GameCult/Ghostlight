//! Sealed replacement world owner under construction.
//!
//! The world owner is one deterministic authority: authenticated commands enter,
//! one reducer decides, and one journal atomically commits the resulting state.
//! Controllers may use models, but models never own lifecycle, scope, affordances,
//! opportunities, reduction, or persistence.

mod action;
mod clock;
mod controllers;
mod elaboration;
mod journal;
mod mailbox;
mod patch;
mod tool_schema;

pub(crate) use action::ActionMismatch;
pub(crate) use clock::{FictionalMinutes, Motion, TickMinutes};
pub(crate) use controllers::{
    ControllerError, ControllerModels, ControllerOpenError, ControllerPendingReason,
    ControllerRunner, ControllerWorkCustody, NarrativeCapture, NarrativeDecision, NarrativePending,
    NarrativeRun, OperationalCapture, OperationalDecision, OperationalPending, OperationalRun,
    SourceRange, SubmissionDisposition, TranslationGapSummary,
};
pub(crate) use mailbox::{ControllerPort, ElaborationPort, MailboxError, WorldMailbox};
pub(crate) use patch::{
    AccessKind, Affordance, AffordanceKindName, Audience, AuthoredSource, AuthorityGrant,
    AuthorityKindName, AuthorityTarget, BoundPrecondition, Bounds, ChannelRecord, Commitment,
    CommitmentKey, CommitmentKind, ComponentOpKind, Confidence, Cost, Declaration,
    DependencyTarget, DraftHandle, EffectSlot, EntityDeclaration, EntityKind, EvidenceRef,
    FactRecord, FactStanding, Forum, GrievanceKindName, JurisdictionKey, Knowledge,
    KnowledgeSource, Mismatch, Office, OfficeName, OutcomeBand, PatchAnswer, Position,
    Precondition, PressureMagnitude, PressureSource, Quantity, Reach, Ref, RefKind, Role, RoleSpec,
    Statement, SubjectDeclaration, WorldPatch, WorldScaleIntent, WorldScaleIntentRef,
};
#[cfg(test)]
use patch::{
    AffordanceDeclaration, AudienceRef, AudienceSpec, ChannelDeclaration, ComponentOp,
    DependencyRef, FactDeclaration, FactStandingRef, ReachRef, RouteDeclaration,
};
#[cfg(test)]
pub(crate) use patch::{AuthorityGrantRef, AuthorityTargetRef};
use patch::{EdgeRecord, EntityRecord, LedgerDelta, ResolvedOp, ResolvedPatch, Site};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[cfg(test)]
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

pub(crate) const STATE_SCHEMA: &str = "ghostlight.world_state.elaboration.v1";
pub(crate) const COMMIT_SCHEMA: &str = "ghostlight.world_commit.elaboration.v1";

/// Compatibility tag derived from [`STATE_SCHEMA`]: the trailing
/// `<family>-<version>` pair (e.g. `foundation-v1`). Callers that publish a
/// compatibility marker alongside the schema string must derive it from here
/// rather than hand-copying a second literal that can drift from the schema.
pub(crate) fn state_schema_compatibility_tag() -> String {
    let mut segments = STATE_SCHEMA.rsplit('.');
    let version = segments.next().unwrap_or_default();
    let family = segments.next().unwrap_or_default();
    format!("{family}-{version}")
}

macro_rules! opaque_uuid {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash,
        )]
        #[serde(transparent)]
        pub(crate) struct $name(Uuid);
    };
}

opaque_uuid!(WorldId);
opaque_uuid!(CommandId);
opaque_uuid!(SubjectId);
opaque_uuid!(EntityId);
opaque_uuid!(EdgeId);
opaque_uuid!(ControllerId);
opaque_uuid!(AffordanceId);
opaque_uuid!(EventId);

impl CommandId {
    fn issue() -> Self {
        Self(Uuid::new_v4())
    }

    pub(crate) fn new() -> Self {
        Self::issue()
    }

    pub(crate) fn parse_uuid(value: &str) -> Result<Self, KernelError> {
        Uuid::parse_str(value)
            .map(Self)
            .map_err(|_| KernelError::InvalidCommandId)
    }

    fn key(self) -> String {
        self.0.to_string()
    }
}

impl WorldId {
    /// A named zero for tests that need a world identity and no world.
    #[cfg(test)]
    pub(super) fn nil_for_test() -> Self {
        Self(Uuid::nil())
    }

    fn issue() -> Self {
        Self(Uuid::new_v4())
    }

    fn key(self) -> String {
        self.0.to_string()
    }
}

// Subject, entity, controller, and affordance IDs are derived, never drawn.
// `patch::derive_id` is the only allocator, and it is called from exactly two
// sites: `patch::resolve_patch` and `action::exercise`. The action lane mints
// exactly one referent per invocation — always an `EntityKind::Fact`, always
// `Claimed` by the acting subject, never caller-named, never any other kind.
// These fixtures exist so tests can name an ID that no partition holds.
#[cfg(test)]
impl SubjectId {
    fn issue() -> Self {
        Self(Uuid::new_v4())
    }
}

#[cfg(test)]
impl EntityId {
    fn issue() -> Self {
        Self(Uuid::new_v4())
    }
}

#[cfg(test)]
impl ControllerId {
    fn issue() -> Self {
        Self(Uuid::new_v4())
    }
}

#[cfg(test)]
impl AffordanceId {
    fn issue() -> Self {
        Self(Uuid::new_v4())
    }
}

impl EventId {
    fn for_command(command_id: CommandId) -> Self {
        Self(command_id.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct PrincipalId(String);

impl PrincipalId {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorldPhase {
    Draft,
    Active,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SubjectKind {
    Person,
    Institution,
    Population,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ControllerMode {
    Human,
    NarrativePersona,
    OperationalAgent,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum NewController {
    Human { principal: PrincipalId },
    NarrativePersona,
    OperationalAgent,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct CreateWorld {
    id: CommandId,
    owner: PrincipalId,
    title: String,
    patch: WorldPatch,
    /// Authored once, here, and never mutated: `CommandBody::AdmitPatch` carries
    /// no intent, so write-once is the command shape rather than a check.
    scale_intent: WorldScaleIntentRef,
}

/// Unattributed creation intent. World ingress derives ownership, controller
/// identity, handles, kinds, and affordances from verified principal evidence.
#[derive(Clone, Debug)]
pub(crate) struct CreateWorldIntent {
    pub(crate) id: CommandId,
    pub(crate) title: String,
    pub(crate) human_subject_label: String,
    pub(crate) narrative_persona_label: Option<String>,
    pub(crate) operational_agent_label: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
enum CallerId {
    Principal(PrincipalId),
    Controller(ControllerId),
    /// An internal capability. Never derived from session evidence, never
    /// minted by ingress, and admitted for exactly one command body.
    System(SystemCapability),
}

/// Internally tagged, because a data-carrying variant cannot join a bare-string
/// encoding without silently changing every commit that already carries
/// `Clock`. The tag is part of the commit digest, so the schema bumps with it.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "capability", rename_all = "snake_case")]
enum SystemCapability {
    Clock,
    /// Authors structure inside one jurisdiction. Never derived from session
    /// evidence, never minted by ingress, never held by a subject.
    Elaborator {
        jurisdiction: JurisdictionKey,
    },
}

#[derive(Clone, Debug)]
struct AuthenticatedCaller {
    caller: CallerId,
}

impl AuthenticatedCaller {
    fn verified_principal(principal: PrincipalId) -> Self {
        Self {
            caller: CallerId::Principal(principal),
        }
    }

    fn verified_controller(controller: ControllerId) -> Self {
        Self {
            caller: CallerId::Controller(controller),
        }
    }

    /// Visible only inside the `world` subtree and called only by
    /// `WorldMailbox::submit_clock`. Runtime ingress cannot reach it.
    fn verified_system(capability: SystemCapability) -> Self {
        Self {
            caller: CallerId::System(capability),
        }
    }

    #[cfg(test)]
    fn fixture(caller: CallerId) -> Self {
        Self { caller }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub(crate) struct DecisionScope {
    pub(crate) subject_id: SubjectId,
}

/// The digest of exactly the components a proposal's verification reads: the
/// scope's controller assignment, its affordance grants, its subject's Position,
/// and the routes incident to that place. Built through the one `digest()`
/// helper over ordered containers, so it is a pure function of committed state.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct ScopeDigest(String);

impl ScopeDigest {
    fn as_str(&self) -> &str {
        &self.0
    }

    #[cfg(test)]
    pub(crate) fn fixture(value: &str) -> Self {
        Self(value.into())
    }
}

#[derive(Serialize)]
struct ScopePreimage<'a> {
    world_id: WorldId,
    subject_id: SubjectId,
    controller: &'a ControllerAssignment,
    /// The granted entries *and their definitions*: preconditions and ceilings
    /// are what admission reads, so a change to a granted entry must change the
    /// digest.
    affordances: BTreeMap<AffordanceId, &'a Affordance>,
    components: &'a ScopeComponents,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DecisionOpportunity {
    pub(crate) world_id: WorldId,
    pub(crate) revision: u64,
    pub(crate) scope_digest: ScopeDigest,
    pub(crate) scope: DecisionScope,
    pub(crate) controller_id: ControllerId,
    pub(crate) controller_mode: ControllerMode,
    pub(crate) affordance_ids: Vec<AffordanceId>,
}

impl DecisionOpportunity {
    pub(crate) fn digest(&self) -> Result<String, KernelError> {
        digest(self)
    }
}

/// The referent a role is bound to. Canonical IDs only: an invocation happens
/// in Active, where no declaration exists, so a draft reference is
/// unrepresentable here and the action lane runs no draft resolution.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "target", content = "id", rename_all = "snake_case")]
pub(crate) enum Target {
    Subject(SubjectId),
    Entity(EntityId),
    Edge(EdgeId),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub(crate) struct RoleBinding {
    pub(crate) role: Role,
    pub(crate) target: Target,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "magnitude", content = "value", rename_all = "snake_case")]
pub(crate) enum Magnitude {
    None,
    Quantity(Quantity),
    Cost(Cost),
}

/// One proposal against one effect slot. It carries no referents: they all come
/// from the slot's roles resolved through the invocation's bindings, so an
/// invocation cannot name a target its affordance declared no role for and the
/// ceiling check is a comparison rather than a graph walk.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProposedEffect {
    pub(crate) slot: usize,
    pub(crate) magnitude: Magnitude,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DecisionInvocation {
    pub(crate) affordance: AffordanceId,
    pub(crate) bindings: Vec<RoleBinding>,
    /// Exactly one entry per slot in the entry, in any order.
    pub(crate) proposed: Vec<ProposedEffect>,
    /// Command input, and the only place an utterance's text enters the kernel.
    /// The committed home of those bytes is `facts[fact].statement`.
    pub(crate) speech: Option<Statement>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct CommandEnvelope {
    id: CommandId,
    world_id: WorldId,
    expected_revision: u64,
    caller: CallerId,
    body: CommandBody,
}

/// Unattributed human command intent. The mailbox derives the caller from
/// live app-session evidence before the reducer sees an envelope.
#[derive(Clone, Debug)]
pub(crate) struct PrincipalCommandIntent {
    pub(crate) id: CommandId,
    pub(crate) world_id: WorldId,
    pub(crate) expected_revision: u64,
    pub(crate) body: CommandBody,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum CommandBody {
    ApproveDraft,
    ActivateWorld,
    ExerciseDecision {
        opportunity: DecisionOpportunity,
        invocation: DecisionInvocation,
    },
    DeclineDecision {
        opportunity: DecisionOpportunity,
    },
    AdmitPatch {
        /// Draft answers nothing. Active must answer a currently derived
        /// boundary or a jurisdiction whose deficit is nonzero, and the commit
        /// must satisfy what it answered.
        answers: Option<PatchAnswer>,
        patch: WorldPatch,
    },
    /// The world's only clock advance. Owner or the clock capability, and the
    /// clock capability may do nothing else.
    AdvanceTime {
        minutes: TickMinutes,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct SubjectState {
    label: String,
    kind: SubjectKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ControllerAssignment {
    Human {
        controller_id: ControllerId,
        principal: PrincipalId,
    },
    NarrativePersona {
        controller_id: ControllerId,
    },
    OperationalAgent {
        controller_id: ControllerId,
    },
}

impl ControllerAssignment {
    fn id(&self) -> ControllerId {
        match self {
            Self::Human { controller_id, .. }
            | Self::NarrativePersona { controller_id }
            | Self::OperationalAgent { controller_id } => *controller_id,
        }
    }

    fn mode(&self) -> ControllerMode {
        match self {
            Self::Human { .. } => ControllerMode::Human,
            Self::NarrativePersona { .. } => ControllerMode::NarrativePersona,
            Self::OperationalAgent { .. } => ControllerMode::OperationalAgent,
        }
    }

    fn expected_caller(&self) -> CallerId {
        match self {
            Self::Human { principal, .. } => CallerId::Principal(principal.clone()),
            Self::NarrativePersona { controller_id } | Self::OperationalAgent { controller_id } => {
                CallerId::Controller(*controller_id)
            }
        }
    }

    fn human_principal(&self) -> Option<&PrincipalId> {
        match self {
            Self::Human { principal, .. } => Some(principal),
            Self::NarrativePersona { .. } | Self::OperationalAgent { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DecisionEvent {
    pub(crate) id: EventId,
    pub(crate) revision: u64,
    pub(crate) scope: DecisionScope,
    pub(crate) controller_id: ControllerId,
    /// The entry exercised. The invocation itself lives on the effect: the
    /// invocation is what the caller said, the event is what the world recorded.
    pub(crate) affordance: AffordanceId,
    /// The claim this act asserted, if it carried speech. Never the text: the
    /// event is the world's record and every reader reads it, so it carries the
    /// fact id and the statement lives in `facts` alone.
    pub(crate) speech: Option<EntityId>,
    /// Index into the entry's `outcome_bands`. Re-derived and compared at apply
    /// and at replay, never trusted.
    pub(crate) band: usize,
    /// The selected band's slots, lowered through the invocation's bindings.
    /// Empty when the selected band names no effects.
    pub(crate) effects: Vec<ResolvedOp>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct WorldState {
    schema: String,
    world_id: WorldId,
    revision: u64,
    phase: WorldPhase,
    owner: PrincipalId,
    title: String,
    draft_approvals: BTreeSet<PrincipalId>,
    subjects: BTreeMap<SubjectId, SubjectState>,
    entities: BTreeMap<EntityId, EntityRecord>,
    edges: BTreeMap<EdgeId, EdgeRecord>,
    positions: BTreeMap<SubjectId, Position>,
    /// What each subject holds. Absence is zero at both levels: no stored
    /// `Quantity(0)` and no empty inner map, so a duplicate holding is
    /// unrepresentable and "nothing" has one shape.
    holdings: BTreeMap<SubjectId, BTreeMap<EntityId, Quantity>>,
    /// What each subject depends on. Never empty for a present key.
    dependencies: BTreeMap<SubjectId, BTreeSet<DependencyTarget>>,
    /// One row per `EntityKind::Fact` entity, and no orphan either way. Facts
    /// are write-once: declaration and `AssertClaim` are the only writers, and
    /// standing never changes after admission.
    facts: BTreeMap<EntityId, FactRecord>,
    /// One row per `EntityKind::Channel` entity, and no orphan either way.
    channels: BTreeMap<EntityId, ChannelRecord>,
    /// What each subject knows. The sole owner of who-heard-what: `events`
    /// records what was done, and this decides what a subject may act on and
    /// what its projection may contain. Never empty for a present key, and
    /// absence is "knows nothing of it" — forgetting removes the key.
    knowledge: BTreeMap<SubjectId, BTreeMap<EntityId, Knowledge>>,
    /// What ground each subject has jurisdiction over, and under which
    /// world-declared kind. Never empty for a present key. No subject holds two
    /// grants of one kind over overlapping ground, so nothing ever arbitrates
    /// between two sources of one permission.
    authority: BTreeMap<SubjectId, BTreeSet<AuthorityGrant>>,
    /// The offices each institution constitutes, and who sits in them. An
    /// incumbent never exercises its institution's opportunity:
    /// `controller_assignments` stays the sole answer to who may call, and an
    /// office answers the different question of what its incumbent has
    /// jurisdiction over.
    selection: BTreeMap<SubjectId, BTreeMap<OfficeName, Office>>,
    /// Where each kind of grievance goes, and who may bring it. Keyed by
    /// neither a subject nor a referent, so it enters no scope digest and is
    /// admission-only state.
    redress: BTreeMap<GrievanceKindName, Forum>,
    /// What each subject has promised. Never empty for a present key.
    commitments: BTreeMap<SubjectId, BTreeMap<CommitmentKey, Commitment>>,
    /// Pressure *on* each subject, keyed target-major then source. Never empty
    /// for a present key; no stored zero magnitude.
    pressures: BTreeMap<SubjectId, BTreeMap<PressureSource, PressureMagnitude>>,
    /// When each subject was last given a decision opportunity it consumed.
    /// Absence means never. The debt term of the attention order and nothing
    /// else.
    last_opportunity_at: BTreeMap<SubjectId, FictionalMinutes>,
    /// Minutes since genesis. The world's only clock, advanced by exactly one
    /// command body and read by no wall clock. It enters `state_digest` and no
    /// `ScopePreimage`: it is world truth, and it is read by no precondition, so
    /// a tick that only advances pressure moves no proposal's binding.
    now: FictionalMinutes,
    /// The authored scale target, written once by genesis and never mutated.
    scale_intent: WorldScaleIntent,
    controller_assignments: BTreeMap<DecisionScope, ControllerAssignment>,
    /// What an affordance *is*. World-authored, Draft-only, written by
    /// `admit_resolved` alone.
    affordance_catalog: BTreeMap<AffordanceId, Affordance>,
    /// Who may exercise which entry. A grant carries no payload, so a duplicate
    /// grant is unrepresentable rather than checked.
    affordance_grants: BTreeMap<DecisionScope, BTreeSet<AffordanceId>>,
    events: Vec<DecisionEvent>,
    state_digest: String,
    last_commit_digest: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WorldEffect {
    WorldCreated {
        owner: PrincipalId,
        title: String,
        resolved: ResolvedPatch,
    },
    PatchAdmitted {
        /// What this commit answered, so `apply_effect` re-decides the same
        /// rule `reduce` decided and then proves the answer was satisfied.
        answers: Option<PatchAnswer>,
        resolved: ResolvedPatch,
    },
    DraftApproved {
        principal: PrincipalId,
    },
    WorldActivated,
    DecisionExercised {
        opportunity: DecisionOpportunity,
        /// The command's own input, kept beside the event rather than inside it:
        /// the invocation is what the caller said, the event is what the world
        /// recorded, and only the first carries text.
        invocation: DecisionInvocation,
        event: DecisionEvent,
    },
    DecisionDeclined {
        opportunity: DecisionOpportunity,
    },
    /// The first effect that mutates state for subjects other than one scope.
    /// Its gate is the clock capability or the owner, and `apply_effect`
    /// re-derives the whole motion, so a forged fulfilment, magnitude, row, or
    /// target is one comparison rather than a clause apiece.
    TimeAdvanced {
        minutes: TickMinutes,
        to: FictionalMinutes,
        motion: Motion,
    },
}

/// The exact raw command admitted by the world owner. This is the sole
/// persistent command authority: IDs, callers, retry identity, and effects are
/// derived from this value rather than copied into independently forgeable
/// commit fields.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "command", rename_all = "snake_case")]
enum CommittedCommand {
    CreateWorld(CreateWorld),
    WorldCommand(CommandEnvelope),
}

impl CommittedCommand {
    fn id(&self) -> CommandId {
        match self {
            Self::CreateWorld(command) => command.id,
            Self::WorldCommand(command) => command.id,
        }
    }

    fn caller(&self) -> CallerId {
        match self {
            Self::CreateWorld(command) => CallerId::Principal(command.owner.clone()),
            Self::WorldCommand(command) => command.caller.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct WorldCommit {
    schema: String,
    world_id: WorldId,
    command: CommittedCommand,
    previous_revision: Option<u64>,
    resulting_revision: u64,
    previous_state_digest: Option<String>,
    resulting_state_digest: String,
    previous_commit_digest: Option<String>,
    effect: WorldEffect,
    committed_at: DateTime<Utc>,
    digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SubjectSnapshot {
    pub(crate) id: SubjectId,
    pub(crate) label: String,
    pub(crate) kind: SubjectKind,
    pub(crate) controller_id: ControllerId,
    pub(crate) controller_mode: ControllerMode,
    pub(crate) human_controller: Option<PrincipalId>,
    pub(crate) affordances: BTreeSet<AffordanceId>,
    pub(crate) position: Option<EntityId>,
    /// Lowered from the one `scope_components` owner, so these are exactly what
    /// the scope digest binds: this subject's own holdings, dependencies, and
    /// grants, the routes incident to its place, and the offices it occupies
    /// with what each lends. A counterparty's components never enter.
    pub(crate) holdings: BTreeMap<EntityId, Quantity>,
    pub(crate) dependencies: BTreeSet<DependencyTarget>,
    pub(crate) incident_routes: Vec<EdgeId>,
    pub(crate) authority: BTreeSet<AuthorityGrant>,
    pub(crate) offices_held: Vec<OfficeSnapshot>,
    /// View-only, and covered by no digest: an institution's own offices, so
    /// its controller can see what it lends and to whom. No precondition reads
    /// it, and installing a warden therefore does not reject the institution's
    /// in-flight proposals — what an institution lends is not what it may do.
    pub(crate) offices_granted: Vec<OfficeSnapshot>,
    /// View-only: the forums whose standing covers this subject, through the
    /// same covering predicate `HasStanding` uses.
    pub(crate) redress: Vec<ForumSnapshot>,
    /// Every fact this subject holds, with the statement resolved. The sole
    /// perception surface: a subject perceives a speech act if and only if it
    /// holds `Knowledge` of that act's fact. Ordered by `spoken_at` then `fact`.
    pub(crate) knowledge: Vec<KnowledgeSnapshot>,
    /// The channels this subject controls, by id. Not their reach: who else is
    /// in earshot is a question about other subjects.
    pub(crate) controls: BTreeSet<EntityId>,
    /// Digest-bound, lowered from the one `scope_components` call `snapshot`
    /// already makes, so view and digest cannot drift.
    pub(crate) commitments: Vec<CommitmentSnapshot>,
    /// Pressure on self. View-only: read by no precondition, so covered by no
    /// digest. Pressure this subject *sources* is excluded — it is another
    /// subject's state, and the actor already sees the commitment that produced
    /// it.
    pub(crate) pressures: Vec<PressureSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommitmentSnapshot {
    pub(crate) key: CommitmentKey,
    pub(crate) kind: CommitmentKind,
    pub(crate) counterparty: Option<SubjectId>,
    pub(crate) due: FictionalMinutes,
    pub(crate) period: Option<TickMinutes>,
    /// Derived in `snapshot` so a controller does not recompute it against a
    /// clock it must be handed anyway.
    pub(crate) past_due: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PressureSnapshot {
    pub(crate) source: PressureSource,
    pub(crate) magnitude: PressureMagnitude,
}

/// What a subject knows of one fact. `standing` carries no `EvidenceRef`: a
/// receipt is non-fictional provenance and never enters a subject-facing
/// surface, so a subject learns *that* a fact is canonical and never which
/// receipt admitted it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KnowledgeSnapshot {
    pub(crate) fact: EntityId,
    pub(crate) statement: Statement,
    pub(crate) standing: FactStandingView,
    pub(crate) confidence: Confidence,
    /// The speaker a subject sees is `Told { by }`. A subject holding a fact it
    /// was never told sees the statement with no speaker, which is correct: it
    /// knows the thing, not the telling.
    pub(crate) source: KnowledgeSource,
    /// The revision of the speech act that minted this claim, when some
    /// committed event asserted it.
    pub(crate) spoken_at: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FactStandingView {
    Canonical,
    Claimed { by: SubjectId },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OfficeSnapshot {
    pub(crate) institution: SubjectId,
    pub(crate) office: OfficeName,
    pub(crate) incumbent: Option<SubjectId>,
    /// What the office actually lends: the institution's live grants whose kind
    /// the office delegates.
    pub(crate) authority: BTreeSet<AuthorityGrant>,
}

/// A forum a subject may petition. It carries no standing: a subject learns
/// that it may bring a grievance, not the boundary of everyone else's standing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ForumSnapshot {
    pub(crate) grievance: GrievanceKindName,
    pub(crate) forum: SubjectId,
}

/// One catalog entry as consumers read it. The whole entry, because every
/// derived surface — tool schemas, signature prose, the typed view's permission
/// block — is a projection of it and a narrower snapshot would be a second
/// vocabulary to keep in step.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct AffordanceSnapshot {
    pub(crate) id: AffordanceId,
    pub(crate) entry: Affordance,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResourceSnapshot {
    pub(crate) id: EntityId,
    pub(crate) label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlaceSnapshot {
    pub(crate) id: EntityId,
    pub(crate) label: String,
    pub(crate) container: Option<EntityId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RouteSnapshot {
    pub(crate) id: EdgeId,
    pub(crate) label: String,
    pub(crate) from: EntityId,
    pub(crate) to: EntityId,
    pub(crate) access: AccessKind,
    pub(crate) cost: Cost,
    pub(crate) open: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WorldSnapshot {
    pub(crate) world_id: WorldId,
    pub(crate) revision: u64,
    pub(crate) phase: WorldPhase,
    pub(crate) owner: PrincipalId,
    pub(crate) title: String,
    pub(crate) draft_approvals: BTreeSet<PrincipalId>,
    pub(crate) required_approvers: BTreeSet<PrincipalId>,
    pub(crate) subjects: Vec<SubjectSnapshot>,
    pub(crate) affordances: Vec<AffordanceSnapshot>,
    pub(crate) places: Vec<PlaceSnapshot>,
    pub(crate) resources: Vec<ResourceSnapshot>,
    pub(crate) routes: Vec<RouteSnapshot>,
    /// Already ordered by pressure, then attention debt, then id: one owner, so
    /// Eve, the mesh projection, and any future driver read the same order and
    /// none computes its own.
    pub(crate) opportunities: Vec<DecisionOpportunity>,
    /// Everyone in a world shares the clock, and no controller can read a `due`
    /// without it.
    pub(crate) now: FictionalMinutes,
    /// The elaborator's surface. There is no global pressure register and no
    /// commitment gazetteer.
    pub(crate) boundaries: Vec<CausalBoundary>,
    pub(crate) scale_deficit: Vec<ScaleDeficitRow>,
    pub(crate) state_digest: String,
    pub(crate) last_commit_digest: Option<String>,
}

/// One committed act as the human operator's story feed reads it. It is the
/// operator surface, not a perception surface: no subject-facing lane receives
/// it, and `SelectedDecision` never sees an event log at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OperatorEvent {
    pub(crate) revision: u64,
    pub(crate) speaker: SubjectId,
    pub(crate) speaker_label: String,
    pub(crate) speech: Option<Statement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommitReceipt {
    pub(crate) command_id: CommandId,
    pub(crate) resulting_revision: u64,
    pub(crate) resulting_state_digest: String,
    pub(crate) commit_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CreationReceipt {
    pub(crate) command_id: CommandId,
    pub(crate) world_id: WorldId,
    pub(crate) resulting_state_digest: String,
    pub(crate) commit_digest: String,
}

impl CreationReceipt {
    fn from_commit(commit: &WorldCommit) -> Result<Self, KernelError> {
        let CommittedCommand::CreateWorld(command) = &commit.command else {
            return Err(KernelError::Invariant(
                "world genesis receipt does not point to a creation command".into(),
            ));
        };
        let WorldEffect::WorldCreated { .. } = &commit.effect else {
            return Err(KernelError::Invariant(
                "world genesis receipt does not point to genesis".into(),
            ));
        };
        Ok(Self {
            command_id: command.id,
            world_id: commit.world_id,
            resulting_state_digest: commit.resulting_state_digest.clone(),
            commit_digest: commit.digest.clone(),
        })
    }
}

impl From<&WorldCommit> for CommitReceipt {
    fn from(commit: &WorldCommit) -> Self {
        Self {
            command_id: commit.command.id(),
            resulting_revision: commit.resulting_revision,
            resulting_state_digest: commit.resulting_state_digest.clone(),
            commit_digest: commit.digest.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SubmitReceipt {
    Applied(CommitReceipt),
    AlreadyApplied(CommitReceipt),
}

#[derive(Debug, Error)]
pub(crate) enum KernelError {
    #[error("command ID must be a UUID")]
    InvalidCommandId,
    #[error("world title must not be empty")]
    EmptyTitle,
    #[error("world owner or human controller principal must be canonical and nonempty")]
    EmptyPrincipal,
    #[error("patch rejected: {0:?}")]
    PatchRejected(Vec<Mismatch>),
    #[error("action rejected: {0:?}")]
    ActionRejected(Vec<ActionMismatch>),
    #[error("command targets another world")]
    WorldMismatch,
    #[error("authenticated caller does not match the command")]
    AuthenticationMismatch,
    #[error("caller does not own this world")]
    Unauthorized,
    #[error("command requires phase {expected:?}, current phase is {actual:?}")]
    WrongPhase {
        expected: WorldPhase,
        actual: WorldPhase,
    },
    #[error("caller is not a required draft approver")]
    NotDraftApprover,
    #[error("caller has already approved this draft")]
    DraftAlreadyApproved,
    #[error("draft is missing required approvals: {0:?}")]
    MissingApprovals(Vec<PrincipalId>),
    #[error("this world derives no such decision opportunity")]
    OpportunityMismatch,
    #[error("decision scope changed since the proposal was bound")]
    ScopeChanged {
        scope: DecisionScope,
        expected: ScopeDigest,
        actual: ScopeDigest,
    },
    #[error("caller does not control this decision scope")]
    ControllerMismatch,
    #[error("decision affordance is not granted by the opportunity")]
    AffordanceDenied,
    #[error("expected revision {expected}, current revision {actual}")]
    RevisionMismatch { expected: u64, actual: u64 },
    #[error("command ID was reused with different content")]
    CommandIdConflict,
    #[error("world creation ID was reused with different content")]
    CreationConflict,
    #[error("world creation target already contains another world")]
    CreationTargetOccupied,
    #[error("world has not been created")]
    WorldNotCreated,
    #[error("opened store contains another world")]
    OpenedWorldMismatch,
    #[error("world ownership is uncertain after command {command_id:?}; drop and reopen")]
    RecoveryRequired { command_id: CommandId },
    #[error("world store path no longer names the owned database; drop and reopen explicitly")]
    OwnershipLost,
    #[error("canonical serialization failed: {0}")]
    Serialization(String),
    #[error("world store operation failed: {0}")]
    Store(String),
    #[error("world journal is corrupt: {0}")]
    CorruptJournal(String),
    #[error("private reducer invariant failed: {0}")]
    Invariant(String),
    #[error("an active patch must answer a derived boundary or a nonzero deficit")]
    AnswerRequired,
    #[error("this world derives no such boundary or deficit")]
    AnswerNotDerived,
    #[error("the admitted patch does not satisfy the boundary or deficit it answered")]
    AnswerNotSatisfied,
}

impl KernelError {
    fn requires_owner_restart(&self) -> bool {
        matches!(
            self,
            Self::RecoveryRequired { .. }
                | Self::OwnershipLost
                | Self::Store(_)
                | Self::CorruptJournal(_)
                | Self::Invariant(_)
        )
    }
}

impl From<journal::JournalError> for KernelError {
    fn from(error: journal::JournalError) -> Self {
        match error {
            journal::JournalError::WorldMismatch => Self::OpenedWorldMismatch,
            journal::JournalError::RecoveryRequired { command_id } => {
                Self::RecoveryRequired { command_id }
            }
            journal::JournalError::OwnershipLost => Self::OwnershipLost,
            journal::JournalError::Store(detail) => Self::Store(detail),
            journal::JournalError::Corrupt(detail) => Self::CorruptJournal(detail),
        }
    }
}

struct WorldKernel {
    state: WorldState,
    journal: journal::WorldJournal,
}

struct PreparedCreation {
    command: CreateWorld,
    world_id: WorldId,
    owner: PrincipalId,
    title: String,
    resolved: ResolvedPatch,
}

fn prepare_creation(
    input: CreateWorld,
    authenticated: &AuthenticatedCaller,
) -> Result<PreparedCreation, KernelError> {
    let CallerId::Principal(authenticated_principal) = &authenticated.caller else {
        return Err(KernelError::AuthenticationMismatch);
    };
    validate_principal(&input.owner)?;
    if &input.owner != authenticated_principal {
        return Err(KernelError::AuthenticationMismatch);
    }
    let title = normalize_title(&input.title)?;
    // The world's own identity is not world structure, and it feeds every
    // derived ID, so it is minted before resolution rather than by it.
    let world_id = WorldId::issue();
    let resolved = patch::resolve_patch(
        &WorldState::empty(world_id, input.owner.clone(), title.clone()),
        input.id,
        &input.patch,
        Some(&input.scale_intent),
    )
    .map_err(KernelError::PatchRejected)?;
    Ok(PreparedCreation {
        world_id,
        owner: input.owner.clone(),
        title,
        resolved,
        command: input,
    })
}

impl WorldKernel {
    fn initialize(
        empty: journal::EmptyWorldJournal,
        prepared: PreparedCreation,
    ) -> Result<(Self, CreationReceipt), KernelError> {
        let world_id = prepared.world_id;
        let effect = WorldEffect::WorldCreated {
            owner: prepared.owner,
            title: prepared.title,
            resolved: prepared.resolved,
        };
        let mut state = WorldState::genesis(world_id, &prepared.command, &effect)?;
        let mut genesis = WorldCommit {
            schema: COMMIT_SCHEMA.into(),
            world_id,
            command: CommittedCommand::CreateWorld(prepared.command),
            previous_revision: None,
            resulting_revision: 0,
            previous_state_digest: None,
            resulting_state_digest: state.state_digest.clone(),
            previous_commit_digest: None,
            effect,
            committed_at: Utc::now(),
            digest: String::new(),
        };
        genesis.digest = commit_digest(&genesis)?;
        state.last_commit_digest = Some(genesis.digest.clone());
        let receipt = CreationReceipt::from_commit(&genesis)?;
        let journal = empty.initialize(&state, &genesis)?;
        Ok((Self { state, journal }, receipt))
    }

    fn retry_creation(&self, prepared: &PreparedCreation) -> Result<CreationReceipt, KernelError> {
        self.journal.ensure_healthy()?;
        let Some(commit) = self.journal.commit_for(prepared.command.id) else {
            return Err(KernelError::CreationTargetOccupied);
        };
        if commit.command != CommittedCommand::CreateWorld(prepared.command.clone())
            || !matches!(&commit.effect, WorldEffect::WorldCreated { .. })
        {
            return Err(KernelError::CreationConflict);
        }
        CreationReceipt::from_commit(commit)
    }

    #[cfg(test)]
    fn create(
        path: impl AsRef<Path>,
        input: CreateWorld,
        authenticated: &AuthenticatedCaller,
    ) -> Result<(Self, CreationReceipt), KernelError> {
        let prepared = prepare_creation(input, authenticated)?;
        match journal::WorldJournal::open_owner(path.as_ref())? {
            journal::JournalOpen::Empty(empty) => Self::initialize(empty, prepared),
            journal::JournalOpen::Live { journal, state } => {
                let kernel = Self { state, journal };
                let receipt = kernel.retry_creation(&prepared)?;
                Ok((kernel, receipt))
            }
        }
    }

    #[cfg(test)]
    fn open(path: impl AsRef<Path>, expected_world_id: WorldId) -> Result<Self, KernelError> {
        let (journal, state) = journal::WorldJournal::open(path.as_ref(), expected_world_id)?;
        Ok(Self { state, journal })
    }

    fn snapshot(&self) -> Result<WorldSnapshot, KernelError> {
        self.journal.ensure_healthy()?;
        snapshot(&self.state)
    }

    /// The human operator's story feed. Separate from `snapshot` so that no
    /// controller lane can reach an unscoped event log: a `WorldSnapshot` field
    /// no consumer may read in full would be a loaded gun with a comment on it.
    fn operator_log(&self) -> Result<Vec<OperatorEvent>, KernelError> {
        self.journal.ensure_healthy()?;
        operator_log(&self.state)
    }

    fn submit(
        &mut self,
        command: CommandEnvelope,
        authenticated: &AuthenticatedCaller,
    ) -> Result<SubmitReceipt, KernelError> {
        let committed_at = Utc::now();
        self.journal.ensure_healthy()?;
        if command.world_id != self.state.world_id {
            return Err(KernelError::WorldMismatch);
        }
        if command.caller != authenticated.caller {
            return Err(KernelError::AuthenticationMismatch);
        }
        // Retry identity is caller plus body. The expected revision is stamped by
        // the mailbox owner rather than supplied by the caller, so it is not part
        // of what makes a command the same command.
        if let Some(commit) = self.committed_command(command.id) {
            let CommittedCommand::WorldCommand(committed) = &commit.command else {
                return Err(KernelError::CommandIdConflict);
            };
            return if committed.caller == command.caller && committed.body == command.body {
                Ok(SubmitReceipt::AlreadyApplied(CommitReceipt::from(commit)))
            } else {
                Err(KernelError::CommandIdConflict)
            };
        }
        if command.expected_revision != self.state.revision {
            return Err(KernelError::RevisionMismatch {
                expected: command.expected_revision,
                actual: self.state.revision,
            });
        }

        let effect = reduce(&self.state, &command)?;
        let mut candidate = self.state.clone();
        apply_effect(&mut candidate, command.id, &command.caller, &effect)?;
        candidate.revision = candidate
            .revision
            .checked_add(1)
            .ok_or_else(|| KernelError::Serialization("world revision overflow".into()))?;
        candidate.state_digest = state_digest(&candidate)?;

        let mut commit = WorldCommit {
            schema: COMMIT_SCHEMA.into(),
            world_id: self.state.world_id,
            command: CommittedCommand::WorldCommand(command),
            previous_revision: Some(self.state.revision),
            resulting_revision: candidate.revision,
            previous_state_digest: Some(self.state.state_digest.clone()),
            resulting_state_digest: candidate.state_digest.clone(),
            previous_commit_digest: self.state.last_commit_digest.clone(),
            effect,
            committed_at,
            digest: String::new(),
        };
        commit.digest = commit_digest(&commit)?;
        candidate.last_commit_digest = Some(commit.digest.clone());

        self.journal.commit(&candidate, &commit)?;
        self.state = candidate;
        Ok(SubmitReceipt::Applied(CommitReceipt::from(&commit)))
    }

    fn committed_command(&self, command_id: CommandId) -> Option<&WorldCommit> {
        self.journal.commit_for(command_id)
    }

    fn controller_receipt(
        &self,
        command_id: CommandId,
        opportunity: &DecisionOpportunity,
        invocation: &DecisionInvocation,
    ) -> Result<Option<CommitReceipt>, KernelError> {
        self.journal.ensure_healthy()?;
        let Some(commit) = self.committed_command(command_id) else {
            return Ok(None);
        };
        let CommittedCommand::WorldCommand(CommandEnvelope {
            body:
                CommandBody::ExerciseDecision {
                    opportunity: committed_opportunity,
                    invocation: committed_invocation,
                },
            ..
        }) = &commit.command
        else {
            return Err(KernelError::CommandIdConflict);
        };
        if committed_opportunity != opportunity || committed_invocation != invocation {
            return Err(KernelError::CommandIdConflict);
        }
        Ok(Some(CommitReceipt::from(commit)))
    }

    fn controller_decline_receipt(
        &self,
        command_id: CommandId,
        opportunity: &DecisionOpportunity,
    ) -> Result<Option<CommitReceipt>, KernelError> {
        self.journal.ensure_healthy()?;
        let Some(commit) = self.committed_command(command_id) else {
            return Ok(None);
        };
        let CommittedCommand::WorldCommand(CommandEnvelope {
            body:
                CommandBody::DeclineDecision {
                    opportunity: committed_opportunity,
                },
            ..
        }) = &commit.command
        else {
            return Err(KernelError::CommandIdConflict);
        };
        if committed_opportunity != opportunity {
            return Err(KernelError::CommandIdConflict);
        }
        Ok(Some(CommitReceipt::from(commit)))
    }
}

fn reduce(state: &WorldState, command: &CommandEnvelope) -> Result<WorldEffect, KernelError> {
    // The one admission rule for internal capabilities, stated here and again
    // at the top of `apply_effect`: `reduce` decides and `apply_effect`
    // re-decides, which is how every gate in this file is written.
    require_system_capability(&command.caller, &command.body)?;
    match &command.body {
        CommandBody::ApproveDraft => {
            require_phase(state, WorldPhase::Draft)?;
            let CallerId::Principal(principal) = &command.caller else {
                return Err(KernelError::NotDraftApprover);
            };
            if !required_approvers(state).contains(principal) {
                return Err(KernelError::NotDraftApprover);
            }
            if state.draft_approvals.contains(principal) {
                return Err(KernelError::DraftAlreadyApproved);
            }
            Ok(WorldEffect::DraftApproved {
                principal: principal.clone(),
            })
        }
        CommandBody::ActivateWorld => {
            require_owner(state, &command.caller)?;
            require_phase(state, WorldPhase::Draft)?;
            let missing: Vec<_> = required_approvers(state)
                .difference(&state.draft_approvals)
                .cloned()
                .collect();
            if !missing.is_empty() {
                return Err(KernelError::MissingApprovals(missing));
            }
            Ok(WorldEffect::WorldActivated)
        }
        CommandBody::ExerciseDecision {
            opportunity,
            invocation,
        } => {
            require_phase(state, WorldPhase::Active)?;
            let current = exact_opportunity(state, opportunity)?;
            let assignment = state
                .controller_assignments
                .get(&current.scope)
                .ok_or(KernelError::OpportunityMismatch)?;
            if assignment.expected_caller() != command.caller {
                return Err(KernelError::ControllerMismatch);
            }
            let granted = require_granted(state, &current, invocation.affordance)?;
            let event = action::exercise(state, command.id, &current, &granted, invocation)?;
            Ok(WorldEffect::DecisionExercised {
                opportunity: current,
                invocation: invocation.clone(),
                event,
            })
        }
        CommandBody::DeclineDecision { opportunity } => {
            require_phase(state, WorldPhase::Active)?;
            let current = exact_opportunity(state, opportunity)?;
            let assignment = state
                .controller_assignments
                .get(&current.scope)
                .ok_or(KernelError::OpportunityMismatch)?;
            if assignment.expected_caller() != command.caller {
                return Err(KernelError::ControllerMismatch);
            }
            Ok(WorldEffect::DecisionDeclined {
                opportunity: current,
            })
        }
        CommandBody::AdvanceTime { minutes } => {
            // There is no time in Draft: a world under construction has no due
            // dates to cross.
            require_phase(state, WorldPhase::Active)?;
            require_clock_caller(state, &command.caller)?;
            let to = state
                .now
                .checked_add(*minutes)
                .ok_or_else(|| KernelError::Serialization("world clock overflow".into()))?;
            // A tick with an empty motion is still a commit: the clock moved,
            // which is a canonical change. That is why the clock is a state
            // field rather than a patch operation.
            Ok(WorldEffect::TimeAdvanced {
                minutes: *minutes,
                to,
                motion: clock::derive_motion(state, to),
            })
        }
        CommandBody::AdmitPatch { answers, patch } => {
            // Answer before authority, because the jurisdiction check reads the
            // answer and a missing one must read as `AnswerRequired` rather
            // than `Unauthorized`. Resolution before confinement, because
            // confinement reads draft-resolved referents and a structurally
            // broken patch should return its complete mismatch set rather than
            // a jurisdiction complaint about a reference that does not resolve.
            require_answer(
                state,
                answers.as_ref(),
                !(patch.declarations.is_empty() && patch.evidence.is_empty()),
            )?;
            let confinement = require_patch_author(state, &command.caller, answers.as_ref())?;
            let resolved = patch::resolve_patch(state, command.id, patch, None)
                .map_err(KernelError::PatchRejected)?;
            if let Some(jurisdiction) = confinement {
                confine_to_jurisdiction(state, &resolved, jurisdiction)
                    .map_err(KernelError::PatchRejected)?;
            }
            Ok(WorldEffect::PatchAdmitted {
                answers: answers.clone(),
                resolved,
            })
        }
    }
}

/// A system capability is admitted for exactly one command body and nothing
/// else. Stated once here, reached by `reduce` and by `apply_effect`.
fn require_system_capability(caller: &CallerId, body: &CommandBody) -> Result<(), KernelError> {
    let CallerId::System(capability) = caller else {
        return Ok(());
    };
    let admitted = matches!(
        (capability, body),
        (SystemCapability::Clock, CommandBody::AdvanceTime { .. })
            | (
                SystemCapability::Elaborator { .. },
                CommandBody::AdmitPatch { .. }
            )
    );
    if admitted {
        Ok(())
    } else {
        Err(KernelError::Unauthorized)
    }
}

/// The owner may tick from Eve; the tick task may tick with no session. Nothing
/// else may tick.
fn require_clock_caller(state: &WorldState, caller: &CallerId) -> Result<(), KernelError> {
    if caller == &CallerId::System(SystemCapability::Clock) {
        Ok(())
    } else {
        require_owner(state, caller)
    }
}

/// Answering is what lets an Active patch declare. Draft answers nothing:
/// `SeedRequest` — the ontology's draft-phase answer — is not inhabited, so
/// genesis and draft elaboration answer nothing. In Active, a patch that only
/// changes components of existing structure is the operator's hand and answers
/// nothing; a patch that declares or admits evidence is elaboration, and
/// elaboration answers a boundary the kernel currently derives or a
/// jurisdiction whose deficit is nonzero.
fn require_answer(
    state: &WorldState,
    answers: Option<&PatchAnswer>,
    declares: bool,
) -> Result<(), KernelError> {
    match (state.phase, answers) {
        (WorldPhase::Draft, None) => Ok(()),
        (WorldPhase::Draft, Some(_)) => Err(KernelError::AnswerNotDerived),
        (WorldPhase::Active, None) if declares => Err(KernelError::AnswerRequired),
        (WorldPhase::Active, None) => Ok(()),
        (WorldPhase::Active, Some(PatchAnswer::Boundary(claimed))) => {
            exact_boundary(state, claimed).map(|_| ())
        }
        (WorldPhase::Active, Some(PatchAnswer::Deficit(jurisdiction))) => {
            if jurisdiction_deficit(state, *jurisdiction)? > 0 {
                Ok(())
            } else {
                Err(KernelError::AnswerNotDerived)
            }
        }
    }
}

/// The total deficit of one jurisdiction, which is what an answer must strictly
/// reduce.
fn jurisdiction_deficit(
    state: &WorldState,
    jurisdiction: JurisdictionKey,
) -> Result<u64, KernelError> {
    Ok(derive_scale_deficit(state)?
        .into_iter()
        .filter(|row| row.jurisdiction == jurisdiction)
        .map(|row| u64::from(row.deficit))
        .sum())
}

/// A human principal joins `required_approvers`, so only the lane that builds
/// revision 0 from nothing may bind one. The predicate reads state, never the
/// caller. It takes the two fields it inspects rather than a whole
/// `WorldState` so the genesis lane, which resolves declarations before any
/// `WorldState` exists, can call the same owner instead of hand-copying the
/// literal it would otherwise always evaluate to.
fn admits_human(revision: u64, subjects: &BTreeMap<SubjectId, SubjectState>) -> bool {
    revision == 0 && subjects.is_empty()
}

impl WorldState {
    /// The world before any structure. Genesis resolves its own patch against
    /// this value, so the genesis lane and `AdmitPatch` share one resolver
    /// signature instead of hand-passing empty partitions.
    fn empty(world_id: WorldId, owner: PrincipalId, title: String) -> Self {
        Self {
            schema: STATE_SCHEMA.into(),
            world_id,
            revision: 0,
            phase: WorldPhase::Draft,
            owner,
            title,
            draft_approvals: BTreeSet::new(),
            subjects: BTreeMap::new(),
            entities: BTreeMap::new(),
            edges: BTreeMap::new(),
            positions: BTreeMap::new(),
            holdings: BTreeMap::new(),
            dependencies: BTreeMap::new(),
            facts: BTreeMap::new(),
            channels: BTreeMap::new(),
            knowledge: BTreeMap::new(),
            authority: BTreeMap::new(),
            selection: BTreeMap::new(),
            redress: BTreeMap::new(),
            commitments: BTreeMap::new(),
            pressures: BTreeMap::new(),
            last_opportunity_at: BTreeMap::new(),
            now: FictionalMinutes::default(),
            scale_intent: WorldScaleIntent::default(),
            controller_assignments: BTreeMap::new(),
            affordance_catalog: BTreeMap::new(),
            affordance_grants: BTreeMap::new(),
            events: Vec::new(),
            state_digest: String::new(),
            last_commit_digest: None,
        }
    }

    fn genesis(
        world_id: WorldId,
        command: &CreateWorld,
        effect: &WorldEffect,
    ) -> Result<Self, KernelError> {
        let WorldEffect::WorldCreated {
            owner,
            title,
            resolved,
        } = effect
        else {
            return Err(KernelError::Invariant(
                "genesis state requires a world-created effect".into(),
            ));
        };
        validate_principal(&command.owner)?;
        let expected_title = normalize_title(&command.title)?;
        if owner != &command.owner || title != &expected_title {
            return Err(KernelError::Invariant(
                "genesis effect does not match the admitted creation command".into(),
            ));
        }
        // The same re-derive-and-compare that `apply_committed_command` runs for
        // every other command. Deterministic allocation is what lets one
        // equality replace a field-by-field binding zip.
        let mut state = Self::empty(world_id, owner.clone(), title.clone());
        let expected = patch::resolve_patch(
            &state,
            command.id,
            &command.patch,
            Some(&command.scale_intent),
        )
        .map_err(KernelError::PatchRejected)?;
        if &expected != resolved {
            return Err(KernelError::Invariant(
                "genesis effect does not derive from its creation command".into(),
            ));
        }
        admit_resolved(&mut state, resolved)?;
        state.state_digest = state_digest(&state)?;
        Ok(state)
    }
}

/// The only writer of every ontology partition. Both admission lanes mutate
/// through it in one fixed order — entities, routes, subjects, operations — so a
/// patch may relocate along a route it declares. It re-derives every structural
/// claim from `state`, so an effect that skipped resolution dies here.
fn admit_resolved(state: &mut WorldState, resolved: &ResolvedPatch) -> Result<(), KernelError> {
    if resolved.declares_nothing() && resolved.operations.is_empty() {
        return Err(KernelError::Invariant(
            "admitted patch carries no canonical change".into(),
        ));
    }
    let humans_admitted = admits_human(state.revision, &state.subjects);
    for entity in &resolved.entities {
        if !patch::is_canonical_text(&entity.entity.label) {
            return Err(KernelError::Invariant(
                "admitted entity label is not canonical".into(),
            ));
        }
        if let Some(container) = entity.entity.container
            && (entity.entity.kind != EntityKind::Place
                || state
                    .entities
                    .get(&container)
                    .is_none_or(|record| record.kind != EntityKind::Place))
        {
            return Err(KernelError::Invariant(
                "admitted container does not name a canonical place".into(),
            ));
        }
        if state
            .entities
            .insert(entity.entity_id, entity.entity.clone())
            .is_some()
        {
            return Err(KernelError::Invariant(
                "admitted entity ID collision".into(),
            ));
        }
        if !patch::containment_terminates(entity.entity_id, &state.entities) {
            return Err(KernelError::Invariant(
                "admitted place contains itself".into(),
            ));
        }
    }
    for route in &resolved.routes {
        let (from, to) = route.edge.endpoints();
        let endpoint_is_place = |entity_id: &EntityId| {
            state
                .entities
                .get(entity_id)
                .is_some_and(|record| record.kind == EntityKind::Place)
        };
        if !patch::is_canonical_text(route.edge.label())
            || !endpoint_is_place(&from)
            || !endpoint_is_place(&to)
            || from == to
            || !patch::is_valid_cost(route.edge.cost())
        {
            return Err(KernelError::Invariant(
                "admitted route is noncanonical or does not join two places".into(),
            ));
        }
        if state
            .edges
            .insert(route.edge_id, route.edge.clone())
            .is_some()
        {
            return Err(KernelError::Invariant("admitted route ID collision".into()));
        }
    }
    let mut controller_ids: BTreeSet<ControllerId> = state
        .controller_assignments
        .values()
        .map(ControllerAssignment::id)
        .collect();
    for entry in &resolved.affordances {
        if state
            .affordance_catalog
            .insert(entry.affordance_id, entry.affordance.clone())
            .is_some()
        {
            return Err(KernelError::Invariant(
                "admitted affordance ID collision".into(),
            ));
        }
    }
    for subject in &resolved.subjects {
        let scope = DecisionScope {
            subject_id: subject.subject_id,
        };
        if !patch::is_canonical_text(&subject.subject.label) {
            return Err(KernelError::Invariant(
                "admitted subject label is not canonical".into(),
            ));
        }
        validate_assignment(&subject.controller)?;
        if matches!(subject.controller, ControllerAssignment::Human { .. }) && !humans_admitted {
            return Err(KernelError::Invariant(
                "only world genesis may bind a human controller".into(),
            ));
        }
        if subject.affordances.is_empty() {
            return Err(KernelError::Invariant(
                "admitted subject has no affordance".into(),
            ));
        }
        if let Some(position) = subject.position
            && state
                .entities
                .get(&position.place)
                .is_none_or(|entity| entity.kind != EntityKind::Place)
        {
            return Err(KernelError::Invariant(
                "admitted position does not name a canonical place".into(),
            ));
        }
        if !controller_ids.insert(subject.controller.id()) {
            return Err(KernelError::Invariant(
                "admitted controller ID collision".into(),
            ));
        }
        if state
            .subjects
            .insert(subject.subject_id, subject.subject.clone())
            .is_some()
            || state
                .controller_assignments
                .insert(scope, subject.controller.clone())
                .is_some()
        {
            return Err(KernelError::Invariant(
                "admitted subject or scope ID collision".into(),
            ));
        }
        if let Some(position) = subject.position
            && state
                .positions
                .insert(subject.subject_id, position)
                .is_some()
        {
            return Err(KernelError::Invariant(
                "admitted position collides with an existing one".into(),
            ));
        }
        for affordance_id in &subject.affordances {
            if !state.affordance_catalog.contains_key(affordance_id) {
                return Err(KernelError::Invariant(
                    "admitted grant names no catalog entry".into(),
                ));
            }
            state
                .affordance_grants
                .entry(scope)
                .or_default()
                .insert(*affordance_id);
        }
    }
    // Facts and channels: one `entities` row and one payload row per
    // declaration, written together so the bijection has one writer.
    for declared in &resolved.facts {
        let claimed_by_a_stranger = match &declared.fact.standing {
            FactStanding::Canonical { evidence } => {
                !(patch::is_canonical_text(evidence.text()) && resolved.evidence.contains(evidence))
            }
            FactStanding::Claimed { by } => !state.subjects.contains_key(by),
        };
        if !patch::is_canonical_text(&declared.entity.label)
            || !patch::is_canonical_text(declared.fact.statement.as_str())
            || declared.entity.kind != EntityKind::Fact
            || claimed_by_a_stranger
        {
            return Err(KernelError::Invariant(
                "admitted fact is noncanonical, unevidenced, or asserted by no live subject".into(),
            ));
        }
        if state
            .entities
            .insert(declared.entity_id, declared.entity.clone())
            .is_some()
            || state
                .facts
                .insert(declared.entity_id, declared.fact.clone())
                .is_some()
        {
            return Err(KernelError::Invariant("admitted fact ID collision".into()));
        }
    }
    for declared in &resolved.channels {
        if !patch::is_canonical_text(&declared.entity.label)
            || declared.entity.kind != EntityKind::Channel
            || !channel_referents_exist(state, &declared.channel)
        {
            return Err(KernelError::Invariant(
                "admitted channel is noncanonical or names no live reach".into(),
            ));
        }
        if state
            .entities
            .insert(declared.entity_id, declared.entity.clone())
            .is_some()
            || state
                .channels
                .insert(declared.entity_id, declared.channel.clone())
                .is_some()
        {
            return Err(KernelError::Invariant(
                "admitted channel ID collision".into(),
            ));
        }
    }
    if let Some(intent) = &resolved.scale_intent {
        state.scale_intent = intent.clone();
    }
    apply_operations(state, &resolved.operations, &resolved.evidence)
}

/// The component writer for every lane: snapshot the before-totals for every
/// resource the operations name, apply each operation, accumulate the ledger
/// deltas, and prove conservation once. Declaration admission calls it after its
/// declaration loops; an exercised affordance's lowered effects call it
/// directly. One writer, one conservation statement.
///
/// A forged effect reaches this function without ever passing the resolver, so
/// conservation is re-derived over the committed partitions, against the same
/// equation, with `before` read from state rather than from anything the effect
/// asserts.
fn apply_operations(
    state: &mut WorldState,
    operations: &[ResolvedOp],
    evidence: &[EvidenceRef],
) -> Result<(), KernelError> {
    let mut deltas: BTreeMap<EntityId, LedgerDelta> = BTreeMap::new();
    for operation in operations {
        for resource in operation_resources(operation) {
            deltas.entry(resource).or_default().before = resource_total(state, resource);
        }
    }
    for operation in operations {
        apply_operation(state, operation, evidence)?;
        match operation {
            ResolvedOp::Transform {
                from_resource,
                into_resource,
                qty,
                ..
            } => {
                deltas.entry(*from_resource).or_default().spent += u128::from(qty.0);
                deltas.entry(*into_resource).or_default().gained += u128::from(qty.0);
            }
            ResolvedOp::Consume { resource, qty, .. } => {
                deltas.entry(*resource).or_default().consumed += u128::from(qty.0);
            }
            ResolvedOp::Admit { resource, qty, .. } => {
                deltas.entry(*resource).or_default().admitted += u128::from(qty.0);
            }
            _ => {}
        }
    }
    for (resource, delta) in &mut deltas {
        delta.after = resource_total(state, *resource);
    }
    if patch::check_ledger(&deltas).is_some() {
        return Err(KernelError::Invariant("custody does not conserve".into()));
    }
    // Disjointness, re-derived over the committed partitions for the same
    // reason conservation is: a forged effect reaches this function without
    // passing the resolver.
    if operations.iter().any(|operation| {
        matches!(
            operation,
            ResolvedOp::GrantAuthority { .. }
                | ResolvedOp::OpenOffice { .. }
                | ResolvedOp::InstallIncumbent { .. }
        )
    }) && overlapping_holder(state).is_some()
    {
        return Err(KernelError::Invariant(
            "one subject holds two overlapping jurisdictions of one kind".into(),
        ));
    }
    Ok(())
}

/// Every resource an operation names, so `admit_resolved` can read its committed
/// total before touching it.
fn operation_resources(operation: &ResolvedOp) -> Vec<EntityId> {
    match operation {
        ResolvedOp::Transfer { resource, .. }
        | ResolvedOp::Consume { resource, .. }
        | ResolvedOp::Admit { resource, .. } => vec![*resource],
        ResolvedOp::Transform {
            from_resource,
            into_resource,
            ..
        } => vec![*from_resource, *into_resource],
        _ => Vec::new(),
    }
}

fn resource_total(state: &WorldState, resource: EntityId) -> u128 {
    state
        .holdings
        .values()
        .filter_map(|held| held.get(&resource))
        .map(|quantity| u128::from(quantity.0))
        .sum()
}

/// What a holder holds. Absence is zero.
fn held(state: &WorldState, holder: SubjectId, resource: EntityId) -> u64 {
    state
        .holdings
        .get(&holder)
        .and_then(|held| held.get(&resource))
        .map_or(0, |quantity| quantity.0)
}

/// Zero removes the resource key, and an emptied holder removes the holder key,
/// so `holdings` has exactly one representation of nothing.
fn set_held(state: &mut WorldState, holder: SubjectId, resource: EntityId, value: u64) {
    if value == 0 {
        let empty = if let Some(held) = state.holdings.get_mut(&holder) {
            held.remove(&resource);
            held.is_empty()
        } else {
            false
        };
        if empty {
            state.holdings.remove(&holder);
        }
    } else {
        state
            .holdings
            .entry(holder)
            .or_default()
            .insert(resource, Quantity(value));
    }
}

/// Whether a subject and a resource are both canonical and of the right kind.
fn custody_referents_exist(state: &WorldState, holder: SubjectId, resource: EntityId) -> bool {
    state.subjects.contains_key(&holder)
        && state
            .entities
            .get(&resource)
            .is_some_and(|record| record.kind == EntityKind::Resource)
}

fn dependency_target_exists(state: &WorldState, target: DependencyTarget) -> bool {
    match target {
        DependencyTarget::Resource(entity_id) => state
            .entities
            .get(&entity_id)
            .is_some_and(|record| record.kind == EntityKind::Resource),
        DependencyTarget::Route(edge_id) => state.edges.contains_key(&edge_id),
        DependencyTarget::Subject(subject_id) => state.subjects.contains_key(&subject_id),
    }
}

/// The component half of admission. Every precondition is re-derived from the
/// partitions, so a forged operation cannot assert a move the topology refuses.
fn apply_operation(
    state: &mut WorldState,
    operation: &ResolvedOp,
    evidence: &[EvidenceRef],
) -> Result<(), KernelError> {
    let insufficient = || KernelError::Invariant("holder does not hold enough".into());
    let overflow = || KernelError::Invariant("holding would overflow".into());
    let unknown = || KernelError::Invariant("custody operation names no canonical referent".into());
    let zero = || KernelError::Invariant("custody operation moves nothing".into());
    match operation {
        ResolvedOp::Relocate {
            subject_id,
            edge_id,
        } => {
            let route = state
                .edges
                .get(edge_id)
                .ok_or_else(|| KernelError::Invariant("relocation names no route".into()))?;
            let (from, to) = route.endpoints();
            let open = route.is_open();
            let access = route.access().clone();
            if !open
                || state.positions.get(subject_id) != Some(&Position { place: from })
                || !patch::route_admits(state, &subject_authority(state, *subject_id), &access, to)
            {
                return Err(KernelError::Invariant(
                    "relocation does not traverse an open route the subject may take from its place"
                        .into(),
                ));
            }
            state.positions.insert(*subject_id, Position { place: to });
        }
        ResolvedOp::OpenRoute { edge_id } | ResolvedOp::CloseRoute { edge_id } => {
            let open = matches!(operation, ResolvedOp::OpenRoute { .. });
            let route = state
                .edges
                .get_mut(edge_id)
                .ok_or_else(|| KernelError::Invariant("route operation names no route".into()))?;
            if route.is_open() == open {
                return Err(KernelError::Invariant(
                    "route operation changes nothing".into(),
                ));
            }
            route.set_open(open);
        }
        ResolvedOp::AlterCost { edge_id, cost } => {
            if !patch::is_valid_cost(*cost) {
                return Err(KernelError::Invariant(
                    "admitted cost is out of range".into(),
                ));
            }
            let route = state
                .edges
                .get_mut(edge_id)
                .ok_or_else(|| KernelError::Invariant("route operation names no route".into()))?;
            if route.cost() == *cost {
                return Err(KernelError::Invariant(
                    "route operation changes nothing".into(),
                ));
            }
            route.set_cost(*cost);
        }
        ResolvedOp::Transfer {
            from,
            to,
            resource,
            qty,
        } => {
            if !custody_referents_exist(state, *from, *resource) || !state.subjects.contains_key(to)
            {
                return Err(unknown());
            }
            if qty.0 == 0 || from == to {
                return Err(zero());
            }
            let remaining = held(state, *from, *resource)
                .checked_sub(qty.0)
                .ok_or_else(insufficient)?;
            let credited = held(state, *to, *resource)
                .checked_add(qty.0)
                .ok_or_else(overflow)?;
            set_held(state, *from, *resource, remaining);
            set_held(state, *to, *resource, credited);
        }
        ResolvedOp::Transform {
            holder,
            from_resource,
            into_resource,
            qty,
        } => {
            if !custody_referents_exist(state, *holder, *from_resource)
                || !custody_referents_exist(state, *holder, *into_resource)
            {
                return Err(unknown());
            }
            if qty.0 == 0 || from_resource == into_resource {
                return Err(zero());
            }
            let remaining = held(state, *holder, *from_resource)
                .checked_sub(qty.0)
                .ok_or_else(insufficient)?;
            let gained = held(state, *holder, *into_resource)
                .checked_add(qty.0)
                .ok_or_else(overflow)?;
            set_held(state, *holder, *from_resource, remaining);
            set_held(state, *holder, *into_resource, gained);
        }
        ResolvedOp::Consume {
            holder,
            resource,
            qty,
        } => {
            if !custody_referents_exist(state, *holder, *resource) {
                return Err(unknown());
            }
            if qty.0 == 0 {
                return Err(zero());
            }
            let remaining = held(state, *holder, *resource)
                .checked_sub(qty.0)
                .ok_or_else(insufficient)?;
            set_held(state, *holder, *resource, remaining);
        }
        ResolvedOp::Admit {
            holder,
            resource,
            qty,
            evidence: cited,
        } => {
            if !custody_referents_exist(state, *holder, *resource) {
                return Err(unknown());
            }
            if qty.0 == 0 {
                return Err(zero());
            }
            if !evidence.contains(cited) {
                return Err(KernelError::Invariant(
                    "admitted quantity cites no evidence in its patch".into(),
                ));
            }
            let admitted = held(state, *holder, *resource)
                .checked_add(qty.0)
                .ok_or_else(overflow)?;
            set_held(state, *holder, *resource, admitted);
        }
        ResolvedOp::Bind { subject, target } | ResolvedOp::Release { subject, target } => {
            let bind = matches!(operation, ResolvedOp::Bind { .. });
            if !state.subjects.contains_key(subject) || !dependency_target_exists(state, *target) {
                return Err(unknown());
            }
            if *target == DependencyTarget::Subject(*subject) {
                return Err(KernelError::Invariant(
                    "a subject cannot depend on itself".into(),
                ));
            }
            let bound = state
                .dependencies
                .get(subject)
                .is_some_and(|targets| targets.contains(target));
            if bound == bind {
                return Err(KernelError::Invariant(
                    "dependency operation changes nothing".into(),
                ));
            }
            if bind {
                state
                    .dependencies
                    .entry(*subject)
                    .or_default()
                    .insert(*target);
            } else {
                let empty = if let Some(targets) = state.dependencies.get_mut(subject) {
                    targets.remove(target);
                    targets.is_empty()
                } else {
                    false
                };
                if empty {
                    state.dependencies.remove(subject);
                }
            }
        }
        ResolvedOp::GrantAuthority { holder, grant }
        | ResolvedOp::RevokeAuthority { holder, grant } => {
            let granting = matches!(operation, ResolvedOp::GrantAuthority { .. });
            if !state.subjects.contains_key(holder)
                || !authority_referent_exists(state, grant.over)
                || !patch::is_civic_name(&grant.kind.0)
            {
                return Err(unknown());
            }
            let held = state
                .authority
                .get(holder)
                .is_some_and(|grants| grants.contains(grant));
            if held == granting {
                return Err(KernelError::Invariant(
                    "authority operation changes nothing".into(),
                ));
            }
            if granting {
                state
                    .authority
                    .entry(*holder)
                    .or_default()
                    .insert(grant.clone());
            } else {
                let empty = if let Some(grants) = state.authority.get_mut(holder) {
                    grants.remove(grant);
                    grants.is_empty()
                } else {
                    false
                };
                if empty {
                    state.authority.remove(holder);
                }
            }
        }
        ResolvedOp::OpenOffice {
            institution,
            office,
            delegated,
        } => {
            require_institution(state, *institution, office)?;
            if delegated.is_empty() || delegated.iter().any(|kind| !patch::is_civic_name(&kind.0)) {
                return Err(KernelError::Invariant(
                    "an office lends no canonical authority kind".into(),
                ));
            }
            let current = state
                .selection
                .get(institution)
                .and_then(|offices| offices.get(office));
            if current.is_some_and(|seat| &seat.delegated == delegated) {
                return Err(KernelError::Invariant(
                    "office operation changes nothing".into(),
                ));
            }
            // A sitting incumbent survives a reconstitution: clipping an
            // office's powers under its holder is a political act, not a
            // vacancy.
            let incumbent = current.and_then(|seat| seat.incumbent);
            state.selection.entry(*institution).or_default().insert(
                office.clone(),
                Office {
                    incumbent,
                    delegated: delegated.clone(),
                },
            );
        }
        ResolvedOp::CloseOffice {
            institution,
            office,
        }
        | ResolvedOp::VacateOffice {
            institution,
            office,
        } => {
            let closing = matches!(operation, ResolvedOp::CloseOffice { .. });
            require_institution(state, *institution, office)?;
            let no_office = || KernelError::Invariant("office operation names no office".into());
            let offices = state.selection.get_mut(institution).ok_or_else(no_office)?;
            let occupied = offices
                .get(office)
                .map(|seat| seat.incumbent.is_some())
                .ok_or_else(no_office)?;
            if closing {
                offices.remove(office);
                let empty = offices.is_empty();
                if empty {
                    state.selection.remove(institution);
                }
            } else if !occupied {
                return Err(KernelError::Invariant(
                    "office operation changes nothing".into(),
                ));
            } else {
                offices.get_mut(office).ok_or_else(no_office)?.incumbent = None;
            }
        }
        ResolvedOp::InstallIncumbent {
            institution,
            office,
            incumbent,
        } => {
            require_institution(state, *institution, office)?;
            if state.subjects.get(incumbent).map(|subject| subject.kind)
                != Some(SubjectKind::Person)
            {
                return Err(KernelError::Invariant(
                    "an office holder is not a person subject".into(),
                ));
            }
            let offices = state
                .selection
                .get_mut(institution)
                .ok_or_else(|| KernelError::Invariant("office operation names no office".into()))?;
            if offices
                .iter()
                .any(|(name, seat)| name != office && seat.incumbent == Some(*incumbent))
            {
                return Err(KernelError::Invariant(
                    "one person holds two offices of one institution".into(),
                ));
            }
            let seat = offices
                .get_mut(office)
                .ok_or_else(|| KernelError::Invariant("office operation names no office".into()))?;
            if seat.incumbent == Some(*incumbent) {
                return Err(KernelError::Invariant(
                    "office operation changes nothing".into(),
                ));
            }
            seat.incumbent = Some(*incumbent);
        }
        ResolvedOp::OpenForum {
            grievance,
            forum,
            standing,
        } => {
            if !state.subjects.contains_key(forum)
                || !authority_referent_exists(state, *standing)
                || !patch::is_civic_name(&grievance.0)
            {
                return Err(unknown());
            }
            let seat = Forum {
                forum: *forum,
                standing: *standing,
            };
            if state.redress.get(grievance) == Some(&seat) {
                return Err(KernelError::Invariant(
                    "forum operation changes nothing".into(),
                ));
            }
            state.redress.insert(grievance.clone(), seat);
        }
        ResolvedOp::CloseForum { grievance } => {
            if state.redress.remove(grievance).is_none() {
                return Err(KernelError::Invariant(
                    "forum operation names no forum".into(),
                ));
            }
        }
        ResolvedOp::AssertClaim {
            fact,
            statement,
            by,
        } => {
            if !state.subjects.contains_key(by) || !patch::is_canonical_text(statement.as_str()) {
                return Err(KernelError::Invariant(
                    "a claim names no live asserter or no canonical statement".into(),
                ));
            }
            if state
                .entities
                .insert(
                    *fact,
                    EntityRecord {
                        label: CLAIM_LABEL.into(),
                        kind: EntityKind::Fact,
                        container: None,
                    },
                )
                .is_some()
                || state
                    .facts
                    .insert(
                        *fact,
                        FactRecord {
                            statement: statement.clone(),
                            standing: FactStanding::Claimed { by: *by },
                        },
                    )
                    .is_some()
            {
                return Err(KernelError::Invariant("minted claim ID collision".into()));
            }
        }
        ResolvedOp::AcquireKnowledge {
            subject,
            fact,
            source,
            confidence,
        } => {
            let Some(record) = state.facts.get(fact) else {
                return Err(KernelError::Invariant(
                    "knowledge operation names no canonical fact".into(),
                ));
            };
            if !state.subjects.contains_key(subject)
                || (*source == AuthoredSource::Evidenced
                    && !matches!(record.standing, FactStanding::Canonical { .. }))
            {
                return Err(KernelError::Invariant(
                    "knowledge operation names no live subject, or evidences a claim".into(),
                ));
            }
            let entry = Knowledge {
                confidence: *confidence,
                source: match source {
                    AuthoredSource::Witnessed => KnowledgeSource::Witnessed,
                    AuthoredSource::Evidenced => KnowledgeSource::Evidenced,
                },
            };
            if state
                .knowledge
                .get(subject)
                .and_then(|held| held.get(fact))
                .is_some_and(|held| *held == entry)
            {
                return Err(KernelError::Invariant(
                    "knowledge operation changes nothing".into(),
                ));
            }
            state
                .knowledge
                .entry(*subject)
                .or_default()
                .insert(*fact, entry);
        }
        ResolvedOp::Forget { subject, fact } => {
            let (removed, emptied) = state
                .knowledge
                .get_mut(subject)
                .map(|held| {
                    let removed = held.remove(fact).is_some();
                    (removed, held.is_empty())
                })
                .unwrap_or((false, false));
            if !removed {
                return Err(KernelError::Invariant(
                    "knowledge operation changes nothing".into(),
                ));
            }
            if emptied {
                state.knowledge.remove(subject);
            }
        }
        ResolvedOp::Communicate { speaker, fact, to } => {
            if !state.subjects.contains_key(speaker) || !state.facts.contains_key(fact) {
                return Err(KernelError::Invariant(
                    "a telling names no live speaker or fact".into(),
                ));
            }
            if !can_broadcast(state, *speaker, to) {
                return Err(KernelError::Invariant(
                    "a speaker is outside its own audience".into(),
                ));
            }
            // An empty fan-out is legal: the delta of a telling is a property of
            // the world — an empty room, a silenced channel — not a defect in
            // the proposal. Speaking alone commits the claim and lands nothing.
            for listener in fan_out(state, *speaker, *fact, to) {
                state.knowledge.entry(listener).or_default().insert(
                    *fact,
                    Knowledge {
                        confidence: Confidence::Believed,
                        source: KnowledgeSource::Told {
                            by: *speaker,
                            via: to.channel(),
                        },
                    },
                );
            }
        }
        ResolvedOp::SetReach { channel, reach } => {
            let current = state
                .channels
                .get(channel)
                .ok_or_else(|| KernelError::Invariant("channel operation names no channel".into()))?
                .clone();
            let record = ChannelRecord {
                reach: reach.clone(),
                controller: current.controller,
            };
            if !channel_referents_exist(state, &record) {
                return Err(KernelError::Invariant(
                    "a reach names no live subject or place".into(),
                ));
            }
            if current.reach == *reach {
                return Err(KernelError::Invariant(
                    "channel operation changes nothing".into(),
                ));
            }
            state.channels.insert(*channel, record);
        }
        ResolvedOp::SetController {
            channel,
            controller,
        } => {
            if controller.is_some_and(|subject_id| !state.subjects.contains_key(&subject_id)) {
                return Err(unknown());
            }
            let seat = state.channels.get_mut(channel).ok_or_else(|| {
                KernelError::Invariant("channel operation names no channel".into())
            })?;
            if seat.controller == *controller {
                return Err(KernelError::Invariant(
                    "channel operation changes nothing".into(),
                ));
            }
            seat.controller = *controller;
        }
        ResolvedOp::CreateCommitment {
            subject,
            key,
            commitment,
        } => {
            if !state.subjects.contains_key(subject)
                || !commitment_is_live(state, *subject, commitment)
            {
                return Err(KernelError::Invariant(
                    "a commitment names no live promisor, counterparty, or check".into(),
                ));
            }
            if state
                .commitments
                .entry(*subject)
                .or_default()
                .insert(*key, commitment.clone())
                .is_some()
            {
                return Err(KernelError::Invariant("commitment key collision".into()));
            }
        }
        ResolvedOp::DischargeCommitment { subject, key } => {
            // One removal, two writes, one owner: the commitment and every
            // pressure row it sourced.
            let discharged = || KernelError::Invariant("discharge names no live commitment".into());
            let held = state.commitments.get_mut(subject).ok_or_else(discharged)?;
            held.remove(key).ok_or_else(discharged)?;
            let empty = held.is_empty();
            if empty {
                state.commitments.remove(subject);
            }
            let sourced = PressureSource::Commitment {
                subject: *subject,
                key: *key,
            };
            state.pressures.retain(|_, held| {
                held.remove(&sourced);
                !held.is_empty()
            });
        }
        ResolvedOp::AdvancePressure { source, target, by }
        | ResolvedOp::ReducePressure { source, target, by } => {
            let advancing = matches!(operation, ResolvedOp::AdvancePressure { .. });
            if !state.subjects.contains_key(target) || !pressure_source_is_live(state, source) {
                return Err(unknown());
            }
            let current = state
                .pressures
                .get(target)
                .and_then(|held| held.get(source))
                .map_or(0, |magnitude| magnitude.0);
            let next = if advancing {
                current.saturating_add(by.0)
            } else {
                current.saturating_sub(by.0)
            };
            if next == current {
                return Err(KernelError::Invariant(
                    "pressure operation changes nothing".into(),
                ));
            }
            set_pressure(state, *target, *source, next);
        }
        ResolvedOp::ResolvePressure { source, target } => {
            if state
                .pressures
                .get(target)
                .is_none_or(|held| !held.contains_key(source))
            {
                return Err(KernelError::Invariant(
                    "pressure operation changes nothing".into(),
                ));
            }
            set_pressure(state, *target, *source, 0);
        }
    }
    Ok(())
}

/// Zero removes the source key, and an emptied target removes the target key,
/// so `pressures` has exactly one representation of nothing.
fn set_pressure(state: &mut WorldState, target: SubjectId, source: PressureSource, value: u32) {
    if value == 0 {
        let empty = if let Some(held) = state.pressures.get_mut(&target) {
            held.remove(&source);
            held.is_empty()
        } else {
            false
        };
        if empty {
            state.pressures.remove(&target);
        }
    } else {
        state
            .pressures
            .entry(target)
            .or_default()
            .insert(source, PressureMagnitude(value));
    }
}

/// Whether every referent a commitment names is live and its shape holds.
/// Re-derived over the committed partitions, because a forged effect reaches
/// `apply_operation` without ever passing the resolver.
fn commitment_is_live(state: &WorldState, subject: SubjectId, commitment: &Commitment) -> bool {
    let counterparty_is_live = match commitment.counterparty {
        Some(counterparty) => {
            counterparty != subject
                && state.subjects.contains_key(&counterparty)
                && commitment.kind != CommitmentKind::Goal
        }
        None => true,
    };
    counterparty_is_live
        && (commitment.kind == CommitmentKind::Routine) == commitment.period.is_some()
        && (commitment.kind == CommitmentKind::Routine || commitment.checks.is_empty())
        && commitment.due > state.now
        && commitment
            .checks
            .iter()
            .all(|check| bound_precondition_is_live(state, check))
}

fn bound_precondition_is_live(state: &WorldState, check: &BoundPrecondition) -> bool {
    let is_kind = |entity_id: &EntityId, kind: EntityKind| {
        state
            .entities
            .get(entity_id)
            .is_some_and(|record| record.kind == kind)
    };
    let audience_is_live = |via: &Audience| match via {
        Audience::Colocated => true,
        Audience::Channel(entity_id) => state.channels.contains_key(entity_id),
    };
    match check {
        BoundPrecondition::Present { at } => is_kind(at, EntityKind::Place),
        BoundPrecondition::Reachable { to, within } => {
            is_kind(to, EntityKind::Place) && patch::is_valid_cost(*within)
        }
        BoundPrecondition::Holds { resource, .. } => is_kind(resource, EntityKind::Resource),
        BoundPrecondition::Authorized { over, kind } => {
            patch::is_civic_name(&kind.0)
                && match over {
                    Target::Subject(subject_id) => state.subjects.contains_key(subject_id),
                    Target::Entity(entity_id) => is_kind(entity_id, EntityKind::Place),
                    Target::Edge(edge_id) => state.edges.contains_key(edge_id),
                }
        }
        BoundPrecondition::HasStanding { grievance } => patch::is_civic_name(&grievance.0),
        BoundPrecondition::Knows { fact, .. } => is_kind(fact, EntityKind::Fact),
        BoundPrecondition::CanBroadcast { via } => audience_is_live(via),
        BoundPrecondition::CanReach { subject, via } => {
            state.subjects.contains_key(subject) && audience_is_live(via)
        }
        BoundPrecondition::Committed { to, .. } => state.subjects.contains_key(to),
    }
}

fn pressure_source_is_live(state: &WorldState, source: &PressureSource) -> bool {
    match source {
        PressureSource::Commitment { subject, key } => state
            .commitments
            .get(subject)
            .is_some_and(|held| held.contains_key(key)),
        PressureSource::Dependency(target) => dependency_target_exists(state, *target),
        PressureSource::Subject(subject_id) => state.subjects.contains_key(subject_id),
    }
}

/// The label every minted claim carries. A label is a name, not a transcript:
/// the statement is never copied into it and the label is never derived from
/// it. Labels resolve no references, so identical labels across every claim
/// are unremarkable. The statement itself lives in `facts[fact].statement`
/// and is also carried inside the committed `AssertClaim` effect on the
/// event that minted it; `facts` is the one readable home a projection may
/// consult, and the event copy is the replay witness, never a second read
/// surface.
const CLAIM_LABEL: &str = "claim";

/// Whether a channel's reach and controller name live structure.
fn channel_referents_exist(state: &WorldState, record: &ChannelRecord) -> bool {
    let reach = match &record.reach {
        Reach::Subjects(members) => members
            .iter()
            .all(|subject_id| state.subjects.contains_key(subject_id)),
        Reach::Place(place) => state
            .entities
            .get(place)
            .is_some_and(|entity| entity.kind == EntityKind::Place),
    };
    reach
        && record
            .controller
            .is_none_or(|subject_id| state.subjects.contains_key(&subject_id))
}

/// The one statement of reach. Called by both speech preconditions, by a
/// `Communicate`'s admission, and by its apply. Nothing else computes an
/// audience.
///
/// Co-location is the actor's exact place, not its subtree: a voice fills a
/// room, not the district containing it. A channel's `Reach::Place` is
/// deliberately wider, because a declared broadcast area is a declared choice.
fn audience(state: &WorldState, actor: SubjectId, of: &Audience) -> BTreeSet<SubjectId> {
    match of {
        Audience::Colocated => {
            let Some(here) = state.positions.get(&actor).copied() else {
                return BTreeSet::new();
            };
            state
                .positions
                .iter()
                .filter(|(_, position)| **position == here)
                .map(|(subject_id, _)| *subject_id)
                .collect()
        }
        Audience::Channel(channel) => {
            let Some(record) = state.channels.get(channel) else {
                return BTreeSet::new();
            };
            match &record.reach {
                Reach::Subjects(members) => members.clone(),
                Reach::Place(root) => state
                    .positions
                    .iter()
                    .filter(|(_, position)| {
                        patch::covers_place(&state.entities, *root, position.place)
                    })
                    .map(|(subject_id, _)| *subject_id)
                    .collect(),
            }
        }
    }
}

/// Whether an actor may broadcast into an audience: either the actor stands
/// inside the declared reach, or — for a channel — the actor is that
/// channel's controller. The controller's privilege is narrow: it lets the
/// horn sound from outside the room, but it does not admit the controller to
/// the audience itself. A controller outside its channel's reach gains no
/// knowledge from its own telling and is not a target another actor can
/// reach through that channel; `audience()` stays the one statement of who is
/// inside, and this is the one statement of who may speak.
fn can_broadcast(state: &WorldState, actor: SubjectId, of: &Audience) -> bool {
    if audience(state, actor, of).contains(&actor) {
        return true;
    }
    match of {
        Audience::Channel(channel) => state
            .channels
            .get(channel)
            .is_some_and(|record| record.controller == Some(actor)),
        Audience::Colocated => false,
    }
}

/// Who gains knowledge from one telling: the audience, less the speaker and less
/// anyone who already holds the fact. A telling never overwrites and never
/// downgrades — the knower owns its own credence, and a speaker who could reset
/// a listener's confidence by repeating itself would own another subject's mind
/// through an effect nobody checked. `Believed` is fixed by the kernel and is
/// not a field on the operation, so a speaker cannot choose how much a listener
/// believes it.
fn fan_out(
    state: &WorldState,
    speaker: SubjectId,
    fact: EntityId,
    to: &Audience,
) -> BTreeSet<SubjectId> {
    audience(state, speaker, to)
        .into_iter()
        .filter(|listener| {
            *listener != speaker
                && !state
                    .knowledge
                    .get(listener)
                    .is_some_and(|held| held.contains_key(&fact))
        })
        .collect()
}

/// Whether an authority target names a live subject or a live place.
fn authority_referent_exists(state: &WorldState, target: AuthorityTarget) -> bool {
    match target {
        AuthorityTarget::Subject(subject_id) => state.subjects.contains_key(&subject_id),
        AuthorityTarget::PlaceSubtree(entity_id) => state
            .entities
            .get(&entity_id)
            .is_some_and(|record| record.kind == EntityKind::Place),
    }
}

/// The shared half of every office operation: a canonical office name on a live
/// institution subject.
fn require_institution(
    state: &WorldState,
    institution: SubjectId,
    office: &OfficeName,
) -> Result<(), KernelError> {
    if !patch::is_civic_name(&office.0) {
        return Err(KernelError::Invariant(
            "office name is not canonical".into(),
        ));
    }
    if state.subjects.get(&institution).map(|subject| subject.kind)
        != Some(SubjectKind::Institution)
    {
        return Err(KernelError::Invariant(
            "an office was opened on a subject that is not an institution".into(),
        ));
    }
    Ok(())
}

/// The subject, if any, whose effective authority holds two grants of one kind
/// over overlapping ground. Re-derived over the committed partitions after every
/// batch that could widen a jurisdiction, so a forged effect cannot install the
/// split-brained state the resolver refuses.
fn overlapping_holder(state: &WorldState) -> Option<SubjectId> {
    let holders: BTreeSet<SubjectId> = state
        .authority
        .keys()
        .copied()
        .chain(
            state
                .selection
                .values()
                .flat_map(|offices| offices.values())
                .filter_map(|office| office.incumbent),
        )
        .collect();
    holders.into_iter().find(|holder| {
        let effective: Vec<AuthorityGrant> =
            subject_authority(state, *holder).into_iter().collect();
        effective.iter().enumerate().any(|(index, one)| {
            effective[index + 1..].iter().any(|other| {
                one.kind == other.kind
                    && patch::targets_overlap(&state.entities, one.over, other.over)
            })
        })
    })
}

/// Exactly the components a subject's verification reads. One owner, consumed by
/// the scope digest and by the snapshot, so the two cannot drift. `routes` is
/// every edge whose `from` or `to` is the subject's place: an inbound route
/// decides who can arrive, and reading too little is a correctness hole while
/// reading too much only costs an extra rejection. `holdings` and `dependencies`
/// are the acting subject's own; a counterparty's holdings do not enter, because
/// a transfer changes both subjects' components and so changes both digests.
/// It also holds what the actor *is* authorized over, and nothing about the
/// world the actor reaches: the target's position, the target's container
/// chain, the forum's state, and any subordinate's components stay out. A
/// realm-wide authority whose preimage held its jurisdiction's occupancy would
/// conflict with every commit in the realm, which is whole-world binding
/// restored by the front door. A target that walks out of a jurisdiction
/// mid-flight therefore fails at `Authorized` rather than rebinding, while a
/// revoked grant is a `ScopeChanged`.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(super) struct ScopeComponents {
    position: Option<Position>,
    routes: BTreeMap<EdgeId, EdgeRecord>,
    holdings: BTreeMap<EntityId, Quantity>,
    dependencies: BTreeSet<DependencyTarget>,
    /// This subject's own grants.
    authority: BTreeSet<AuthorityGrant>,
    /// What each held office lends: for every office whose incumbent is this
    /// subject, the institution's live grants whose kind that office delegates.
    /// The institution's authority is copied in through the office link, so one
    /// scope digest covers the grant, the office, and the institution's
    /// jurisdiction, and only delegated kinds enter — an institution's `judge`
    /// jurisdiction never churns a `levy`-only incumbent's proposals.
    delegated: BTreeMap<SubjectId, BTreeMap<OfficeName, BTreeSet<AuthorityGrant>>>,
    /// The keys of this subject's own knowledge. Keys, not payloads: a statement
    /// is immutable, so it cannot change without the key changing, and
    /// confidence is admission-only — a re-appraisal that lowers a subject's own
    /// confidence does not reject its own in-flight proposal, because `Knows`
    /// re-checks at commit and fails closed with `FactUnknown`.
    knows: BTreeSet<EntityId>,
    /// The channels this subject controls, with their records. Membership of a
    /// channel it merely hears never enters: a 1,000-member channel would churn
    /// a thousand digests per membership change, which is whole-world binding by
    /// the front door.
    controls: BTreeMap<EntityId, ChannelRecord>,
    /// This subject's own commitments. `Precondition::Committed` reads them, so
    /// the digest must bind them. Pressure, the clock, and the attention stamp
    /// stay out: no precondition reads any of the three, so a tick that only
    /// advances pressure invalidates no in-flight proposal.
    commitments: BTreeMap<CommitmentKey, Commitment>,
}

/// Whether an authority target covers a referent. The one statement of
/// jurisdictional membership, reached by `Authorized`, `HasStanding`,
/// `route_admits`, the delegation monotonicity rule, and the snapshot's redress
/// projection.
///
/// A place subtree covers a subject standing anywhere under it, a place under
/// it, and a route with either endpoint under it — a city must be able to close
/// its own gate, and a gate's far end is outside the city by construction. A
/// resource, fact, or channel is covered by nothing; `RoleKindUnfit` refuses an
/// `Authorized` over such a role at declaration.
fn covers(state: &WorldState, target: AuthorityTarget, of: Target) -> bool {
    let under = |place: EntityId, root: EntityId| patch::covers_place(&state.entities, root, place);
    match (target, of) {
        (AuthorityTarget::Subject(holder), Target::Subject(subject_id)) => holder == subject_id,
        (AuthorityTarget::Subject(_), _) => false,
        (AuthorityTarget::PlaceSubtree(root), Target::Subject(subject_id)) => state
            .positions
            .get(&subject_id)
            .is_some_and(|position| under(position.place, root)),
        (AuthorityTarget::PlaceSubtree(root), Target::Entity(entity_id)) => state
            .entities
            .get(&entity_id)
            .is_some_and(|record| record.kind == EntityKind::Place && under(entity_id, root)),
        (AuthorityTarget::PlaceSubtree(root), Target::Edge(edge_id)) => {
            state.edges.get(&edge_id).is_some_and(|record| {
                let (from, to) = record.endpoints();
                under(from, root) || under(to, root)
            })
        }
    }
}

/// Own grants plus what every held office lends. The single statement of what a
/// subject may invoke authority over. There is no precedence between a direct
/// grant and a delegated one, because `OverlappingJurisdiction` makes it
/// impossible for both to answer for one kind over overlapping ground.
fn effective_authority(
    own: &BTreeSet<AuthorityGrant>,
    delegated: &BTreeMap<SubjectId, BTreeMap<OfficeName, BTreeSet<AuthorityGrant>>>,
) -> BTreeSet<AuthorityGrant> {
    let mut effective = own.clone();
    for lent in delegated.values().flat_map(|offices| offices.values()) {
        effective.extend(lent.iter().cloned());
    }
    effective
}

/// Every office this subject occupies, keyed by institution, with what each one
/// lends of that institution's live grants.
fn delegated_authority(
    state: &WorldState,
    subject_id: SubjectId,
) -> BTreeMap<SubjectId, BTreeMap<OfficeName, BTreeSet<AuthorityGrant>>> {
    let mut held: BTreeMap<SubjectId, BTreeMap<OfficeName, BTreeSet<AuthorityGrant>>> =
        BTreeMap::new();
    for (institution, offices) in &state.selection {
        for (name, office) in offices {
            if office.incumbent != Some(subject_id) {
                continue;
            }
            let lent: BTreeSet<AuthorityGrant> = state
                .authority
                .get(institution)
                .into_iter()
                .flatten()
                .filter(|grant| office.delegated.contains(&grant.kind))
                .cloned()
                .collect();
            held.entry(*institution)
                .or_default()
                .insert(name.clone(), lent);
        }
    }
    held
}

/// The effective authority of one subject read straight from the partitions,
/// for the reducer paths that do not already hold a `ScopeComponents`.
fn subject_authority(state: &WorldState, subject_id: SubjectId) -> BTreeSet<AuthorityGrant> {
    let empty = BTreeSet::new();
    effective_authority(
        state.authority.get(&subject_id).unwrap_or(&empty),
        &delegated_authority(state, subject_id),
    )
}

fn scope_components(state: &WorldState, subject_id: SubjectId) -> ScopeComponents {
    let position = state.positions.get(&subject_id).copied();
    let routes = position
        .map(|position| {
            state
                .edges
                .iter()
                .filter(|(_, record)| {
                    let (from, to) = record.endpoints();
                    from == position.place || to == position.place
                })
                .map(|(edge_id, record)| (*edge_id, record.clone()))
                .collect()
        })
        .unwrap_or_default();
    ScopeComponents {
        position,
        routes,
        holdings: state.holdings.get(&subject_id).cloned().unwrap_or_default(),
        dependencies: state
            .dependencies
            .get(&subject_id)
            .cloned()
            .unwrap_or_default(),
        authority: state
            .authority
            .get(&subject_id)
            .cloned()
            .unwrap_or_default(),
        delegated: delegated_authority(state, subject_id),
        knows: state
            .knowledge
            .get(&subject_id)
            .into_iter()
            .flat_map(BTreeMap::keys)
            .copied()
            .collect(),
        controls: state
            .channels
            .iter()
            .filter(|(_, record)| record.controller == Some(subject_id))
            .map(|(entity_id, record)| (*entity_id, record.clone()))
            .collect(),
        commitments: state
            .commitments
            .get(&subject_id)
            .cloned()
            .unwrap_or_default(),
    }
}

/// The human operator's story feed, derived from the causal record. The events
/// log is not a perception surface: `knowledge` is the sole answer to what a
/// subject may act on and what its projection may contain.
fn operator_log(state: &WorldState) -> Result<Vec<OperatorEvent>, KernelError> {
    state
        .events
        .iter()
        .map(|event| {
            let speaker = state
                .subjects
                .get(&event.scope.subject_id)
                .ok_or_else(|| KernelError::Invariant("event names no live subject".into()))?;
            Ok(OperatorEvent {
                revision: event.revision,
                speaker: event.scope.subject_id,
                speaker_label: speaker.label.clone(),
                speech: event
                    .speech
                    .and_then(|fact| state.facts.get(&fact))
                    .map(|record| record.statement.clone()),
            })
        })
        .collect()
}

fn snapshot(state: &WorldState) -> Result<WorldSnapshot, KernelError> {
    // One pass over the causal record so each subject's own knowledge map can be
    // stamped with the revision that minted each claim it holds.
    let spoken_at: BTreeMap<EntityId, u64> = state
        .events
        .iter()
        .filter_map(|event| event.speech.map(|fact| (fact, event.revision)))
        .collect();
    let subjects = state
        .subjects
        .iter()
        .map(|(subject_id, subject)| {
            let scope = DecisionScope {
                subject_id: *subject_id,
            };
            let controller = state.controller_assignments.get(&scope).ok_or_else(|| {
                KernelError::Invariant("subject has no controller assignment".into())
            })?;
            let affordances = state
                .affordance_grants
                .get(&scope)
                .cloned()
                .unwrap_or_default();
            let components = scope_components(state, *subject_id);
            let offices_held = components
                .delegated
                .iter()
                .flat_map(|(institution, offices)| {
                    offices
                        .iter()
                        .map(move |(office, authority)| OfficeSnapshot {
                            institution: *institution,
                            office: office.clone(),
                            incumbent: Some(*subject_id),
                            authority: authority.clone(),
                        })
                })
                .collect();
            let offices_granted = state
                .selection
                .get(subject_id)
                .into_iter()
                .flatten()
                .map(|(office, seat)| OfficeSnapshot {
                    institution: *subject_id,
                    office: office.clone(),
                    incumbent: seat.incumbent,
                    authority: state
                        .authority
                        .get(subject_id)
                        .into_iter()
                        .flatten()
                        .filter(|grant| seat.delegated.contains(&grant.kind))
                        .cloned()
                        .collect(),
                })
                .collect();
            let redress = state
                .redress
                .iter()
                .filter(|(_, forum)| covers(state, forum.standing, Target::Subject(*subject_id)))
                .map(|(grievance, forum)| ForumSnapshot {
                    grievance: grievance.clone(),
                    forum: forum.forum,
                })
                .collect();
            // The sole perception surface, derived here rather than in a
            // controller: a controller that recomputed perception would be a
            // second answer to who heard what.
            let mut knowledge: Vec<KnowledgeSnapshot> = state
                .knowledge
                .get(subject_id)
                .into_iter()
                .flatten()
                .map(|(fact, held)| {
                    let record = state.facts.get(fact).ok_or_else(|| {
                        KernelError::Invariant("knowledge names no canonical fact".into())
                    })?;
                    Ok(KnowledgeSnapshot {
                        fact: *fact,
                        statement: record.statement.clone(),
                        standing: match &record.standing {
                            FactStanding::Canonical { .. } => FactStandingView::Canonical,
                            FactStanding::Claimed { by } => FactStandingView::Claimed { by: *by },
                        },
                        confidence: held.confidence,
                        source: held.source,
                        spoken_at: spoken_at.get(fact).copied(),
                    })
                })
                .collect::<Result<Vec<_>, KernelError>>()?;
            knowledge.sort_by_key(|entry| (entry.spoken_at, entry.fact));
            Ok(SubjectSnapshot {
                id: *subject_id,
                label: subject.label.clone(),
                kind: subject.kind,
                controller_id: controller.id(),
                controller_mode: controller.mode(),
                human_controller: controller.human_principal().cloned(),
                affordances,
                position: components.position.map(|position| position.place),
                holdings: components.holdings,
                dependencies: components.dependencies,
                incident_routes: components.routes.into_keys().collect(),
                authority: components.authority,
                offices_held,
                offices_granted,
                redress,
                knowledge,
                controls: components.controls.into_keys().collect(),
                commitments: components
                    .commitments
                    .into_iter()
                    .map(|(key, commitment)| CommitmentSnapshot {
                        key,
                        kind: commitment.kind,
                        counterparty: commitment.counterparty,
                        due: commitment.due,
                        period: commitment.period,
                        past_due: commitment.due <= state.now,
                    })
                    .collect(),
                pressures: state
                    .pressures
                    .get(subject_id)
                    .into_iter()
                    .flatten()
                    .map(|(source, magnitude)| PressureSnapshot {
                        source: *source,
                        magnitude: *magnitude,
                    })
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, KernelError>>()?;
    let places = state
        .entities
        .iter()
        .filter(|(_, record)| record.kind == EntityKind::Place)
        .map(|(entity_id, record)| PlaceSnapshot {
            id: *entity_id,
            label: record.label.clone(),
            container: record.container,
        })
        .collect();
    let resources = state
        .entities
        .iter()
        .filter(|(_, record)| record.kind == EntityKind::Resource)
        .map(|(entity_id, record)| ResourceSnapshot {
            id: *entity_id,
            label: record.label.clone(),
        })
        .collect();
    let routes = state
        .edges
        .iter()
        .map(|(edge_id, record)| {
            let (from, to) = record.endpoints();
            RouteSnapshot {
                id: *edge_id,
                label: record.label().to_owned(),
                from,
                to,
                access: record.access().clone(),
                cost: record.cost(),
                open: record.is_open(),
            }
        })
        .collect();
    Ok(WorldSnapshot {
        world_id: state.world_id,
        revision: state.revision,
        phase: state.phase,
        owner: state.owner.clone(),
        title: state.title.clone(),
        draft_approvals: state.draft_approvals.clone(),
        required_approvers: required_approvers(state),
        subjects,
        affordances: state
            .affordance_catalog
            .iter()
            .map(|(affordance_id, entry)| AffordanceSnapshot {
                id: *affordance_id,
                entry: entry.clone(),
            })
            .collect(),
        places,
        resources,
        routes,
        opportunities: order_opportunities(state, derive_opportunities(state)?),
        now: state.now,
        boundaries: derive_boundaries(state)?,
        scale_deficit: derive_scale_deficit(state)?,
        state_digest: state.state_digest.clone(),
        last_commit_digest: state.last_commit_digest.clone(),
    })
}

/// A place in the world's causal reach that the world has not yet grown into.
/// Derived, never stored, and cleared by no writer: a boundary stops being
/// derived when its predicate stops holding.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "boundary", rename_all = "snake_case")]
pub(crate) enum CausalBoundary {
    UnelaboratedDestination {
        route: EdgeId,
        place: EntityId,
        scope: BoundaryDigest,
    },
    MissingStructure {
        subject: SubjectId,
        key: CommitmentKey,
        scope: BoundaryDigest,
    },
    /// Declared, never derived: no relations partition exists. Answering one is
    /// `AnswerNotDerived`.
    PolityInCausalRange {
        subject: SubjectId,
        scope: BoundaryDigest,
    },
    /// Declared, never derived: no population membership and no slice concept
    /// exists. Answering one is `AnswerNotDerived`.
    IndividuationRequired {
        population: SubjectId,
        scope: BoundaryDigest,
    },
}

/// The digest of exactly the components that derive one boundary, produced by
/// the one `digest()` owner, in the `ScopeDigest` idiom.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct BoundaryDigest(String);

impl BoundaryDigest {
    /// The deficit lane's answer has no preimage of its own: its digest is the
    /// session's, produced by the same `sha256:` spelling.
    pub(super) fn from_digest(value: String) -> Self {
        Self(value)
    }
}

#[derive(Serialize)]
struct UnelaboratedDestinationPreimage<'a> {
    world_id: WorldId,
    route: &'a EdgeRecord,
    place: &'a EntityRecord,
    contained: BTreeSet<EntityId>,
    occupants: BTreeSet<SubjectId>,
    incident: BTreeSet<EdgeId>,
}

#[derive(Serialize)]
struct MissingStructurePreimage<'a> {
    world_id: WorldId,
    subject: SubjectId,
    key: CommitmentKey,
    commitment: &'a Commitment,
    counterparty_authority: BTreeSet<AuthorityGrant>,
    redress: &'a BTreeMap<GrievanceKindName, Forum>,
}

/// Beside `derive_opportunities`, and reached from `snapshot` and from
/// `reduce`'s `AdmitPatch` arm. The same shape as the opportunity derivation,
/// not the same function: the inputs and the consumers differ, and a shared
/// derivation returning a sum type would be a generic helper wearing a name.
fn derive_boundaries(state: &WorldState) -> Result<Vec<CausalBoundary>, KernelError> {
    let mut boundaries = Vec::new();
    for (edge_id, record) in &state.edges {
        let (from, to) = record.endpoints();
        for place in [from, to] {
            let contained: BTreeSet<EntityId> = state
                .entities
                .iter()
                .filter(|(_, entity)| entity.container == Some(place))
                .map(|(entity_id, _)| *entity_id)
                .collect();
            let occupants: BTreeSet<SubjectId> = state
                .positions
                .iter()
                .filter(|(_, position)| position.place == place)
                .map(|(subject_id, _)| *subject_id)
                .collect();
            let incident: BTreeSet<EdgeId> = state
                .edges
                .iter()
                .filter(|(_, candidate)| {
                    let (candidate_from, candidate_to) = candidate.endpoints();
                    candidate_from == place || candidate_to == place
                })
                .map(|(candidate_id, _)| *candidate_id)
                .collect();
            if !contained.is_empty()
                || !occupants.is_empty()
                || incident.len() != 1
                || !incident.contains(edge_id)
            {
                continue;
            }
            let entity = state
                .entities
                .get(&place)
                .ok_or_else(|| KernelError::Invariant("a route names no canonical place".into()))?;
            boundaries.push(CausalBoundary::UnelaboratedDestination {
                route: *edge_id,
                place,
                scope: BoundaryDigest(digest(&UnelaboratedDestinationPreimage {
                    world_id: state.world_id,
                    route: record,
                    place: entity,
                    contained,
                    occupants,
                    incident,
                })?),
            });
        }
    }
    // A commitment the counterparty can neither command nor litigate. Both
    // clauses read the civic partitions through their own predicates, so
    // jurisdiction lent by an office counts and no second covering rule exists.
    // A `Goal` derives nothing: a promise to oneself needs no forum.
    for (subject, held) in &state.commitments {
        for (key, commitment) in held {
            let Some(counterparty) = commitment.counterparty else {
                continue;
            };
            let counterparty_authority = subject_authority(state, counterparty);
            let commandable = counterparty_authority
                .iter()
                .any(|grant| covers(state, grant.over, Target::Subject(*subject)));
            let litigable = state
                .redress
                .values()
                .any(|forum| covers(state, forum.standing, Target::Subject(counterparty)));
            if commandable || litigable {
                continue;
            }
            boundaries.push(CausalBoundary::MissingStructure {
                subject: *subject,
                key: *key,
                scope: BoundaryDigest(digest(&MissingStructurePreimage {
                    world_id: state.world_id,
                    subject: *subject,
                    key: *key,
                    commitment,
                    counterparty_authority,
                    redress: &state.redress,
                })?),
            });
        }
    }
    Ok(boundaries)
}

/// The sole validator of a claimed boundary, mirroring `exact_opportunity`.
fn exact_boundary(
    state: &WorldState,
    claimed: &CausalBoundary,
) -> Result<CausalBoundary, KernelError> {
    derive_boundaries(state)?
        .into_iter()
        .find(|current| current == claimed)
        .ok_or(KernelError::AnswerNotDerived)
}

/// One region's count for one subject kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ScaleDeficitRow {
    pub(crate) jurisdiction: JurisdictionKey,
    pub(crate) kind: SubjectKind,
    pub(crate) target: u32,
    pub(crate) qualified: u32,
    pub(crate) deficit: u32,
}

/// Whether a subject counts toward the world's scale target. Three of the four
/// clauses cannot currently fail — the world-level phase gate, the one-
/// controller rule, and the non-empty grant set are already enforced elsewhere
/// — so `Goal` carries the whole discrimination at this pass. The conjunction
/// is written once anyway, because retirement and grant revocation make two of
/// the clauses live later and a scattered count would then be wrong in three
/// places. "Executable" is structural, never "preconditions currently hold":
/// evaluating them per subject per snapshot would make the deficit flicker as
/// routes open and close.
fn qualifies(state: &WorldState, subject_id: SubjectId) -> bool {
    let scope = DecisionScope { subject_id };
    state.phase == WorldPhase::Active
        && state.controller_assignments.contains_key(&scope)
        && state
            .affordance_grants
            .get(&scope)
            .is_some_and(|granted| !granted.is_empty())
        && state.commitments.get(&subject_id).is_some_and(|held| {
            held.values()
                .any(|commitment| commitment.kind == CommitmentKind::Goal)
        })
}

/// Derived in `snapshot`, never stored. `snapshot` is the recount: it already
/// re-derives everything, and a second after-every-commit step would be a
/// second owner. Only admitted subjects reduce it, so a rejected patch leaves
/// it visible, which is free — a rejection mutates nothing.
fn derive_scale_deficit(state: &WorldState) -> Result<Vec<ScaleDeficitRow>, KernelError> {
    let mut counted: BTreeMap<(JurisdictionKey, SubjectKind), u32> = BTreeMap::new();
    for (subject_id, subject) in &state.subjects {
        if !qualifies(state, *subject_id) {
            continue;
        }
        match state.positions.get(subject_id) {
            // A subject under nested roots counts toward both: layered
            // jurisdiction applied to counting. A subject standing somewhere
            // no declared root covers still counts, in `Uncovered`, so the
            // deficit stays total over every qualifying subject.
            Some(position) => {
                let mut covered = false;
                for root in state.scale_intent.jurisdictions.keys() {
                    if patch::covers_place(&state.entities, *root, position.place) {
                        covered = true;
                        *counted
                            .entry((JurisdictionKey::PlaceSubtree(*root), subject.kind))
                            .or_default() += 1;
                    }
                }
                if !covered {
                    *counted
                        .entry((JurisdictionKey::Uncovered, subject.kind))
                        .or_default() += 1;
                }
            }
            // Counted, visible, and reducing no target.
            None => {
                *counted
                    .entry((JurisdictionKey::Uncovered, subject.kind))
                    .or_default() += 1;
            }
        }
    }
    let mut rows: BTreeMap<(JurisdictionKey, SubjectKind), ScaleDeficitRow> = BTreeMap::new();
    for (kind, world_target) in &state.scale_intent.targets {
        for (root, permille) in &state.scale_intent.jurisdictions {
            let jurisdiction = JurisdictionKey::PlaceSubtree(*root);
            let target = u32::try_from(
                u64::from(*world_target)
                    .checked_mul(u64::from(*permille))
                    .ok_or_else(|| KernelError::Invariant("scale target overflow".into()))?
                    / 1000,
            )
            .map_err(|_| KernelError::Invariant("scale target overflow".into()))?;
            let qualified = counted
                .get(&(jurisdiction, *kind))
                .copied()
                .unwrap_or_default();
            rows.insert(
                (jurisdiction, *kind),
                ScaleDeficitRow {
                    jurisdiction,
                    kind: *kind,
                    target,
                    qualified,
                    deficit: target.saturating_sub(qualified),
                },
            );
        }
    }
    for ((jurisdiction, kind), qualified) in counted {
        rows.entry((jurisdiction, kind))
            .or_insert(ScaleDeficitRow {
                jurisdiction,
                kind,
                target: 0,
                qualified,
                deficit: 0,
            })
            .qualified = qualified;
    }
    Ok(rows
        .into_values()
        .filter(|row| row.target > 0 || row.qualified > 0)
        .collect())
}

/// A pure re-ordering of `derive_opportunities`' output: same values, same
/// length, same set. Never a filter — filtering by readiness would drag
/// precondition reads into `ScopePreimage`, and a tick could then withdraw an
/// opportunity a controller is mid-inference on, which `exact_opportunity`
/// would report as `OpportunityMismatch` rather than the honest `ScopeChanged`.
///
/// A subject never attended sorts ahead of every subject that has, and a
/// subject with no pressure still climbs one tick at a time, so the order
/// starves nobody. `SubjectId` is the final tiebreak, so the order is total and
/// identical on every machine.
fn order_opportunities(
    state: &WorldState,
    mut opportunities: Vec<DecisionOpportunity>,
) -> Vec<DecisionOpportunity> {
    opportunities.sort_by_key(|opportunity| {
        let subject_id = opportunity.scope.subject_id;
        (
            std::cmp::Reverse(pressure_total(state, subject_id)),
            std::cmp::Reverse(attention_debt(state, subject_id)),
            subject_id,
        )
    });
    opportunities
}

fn pressure_total(state: &WorldState, subject_id: SubjectId) -> u64 {
    state
        .pressures
        .get(&subject_id)
        .into_iter()
        .flat_map(BTreeMap::values)
        .fold(0u64, |total, magnitude| {
            total.saturating_add(u64::from(magnitude.0))
        })
}

/// Minutes since this subject last consumed an opportunity. Absence is
/// `u64::MAX`: never attended outranks every subject that has.
fn attention_debt(state: &WorldState, subject_id: SubjectId) -> u64 {
    state
        .last_opportunity_at
        .get(&subject_id)
        .map_or(u64::MAX, |stamp| state.now.since(*stamp))
}

fn derive_opportunities(state: &WorldState) -> Result<Vec<DecisionOpportunity>, KernelError> {
    if state.phase != WorldPhase::Active {
        return Ok(Vec::new());
    }
    state
        .controller_assignments
        .iter()
        .map(|(scope, controller)| {
            let affordance_ids: Vec<_> = state
                .affordance_grants
                .get(scope)
                .into_iter()
                .flatten()
                .copied()
                .collect();
            if affordance_ids.is_empty() {
                return Err(KernelError::Invariant(
                    "active decision scope has no affordances".into(),
                ));
            }
            Ok(DecisionOpportunity {
                world_id: state.world_id,
                revision: state.revision,
                scope_digest: scope_digest(state, *scope)?,
                scope: *scope,
                controller_id: controller.id(),
                controller_mode: controller.mode(),
                affordance_ids,
            })
        })
        .collect()
}

/// The entries one scope is granted, paired with their definitions. A grant
/// naming no catalog entry is a corrupt kernel rather than a rejection: one
/// owner writes both partitions in one commit.
fn granted_entries(
    state: &WorldState,
    scope: DecisionScope,
) -> Result<BTreeMap<AffordanceId, &Affordance>, KernelError> {
    state
        .affordance_grants
        .get(&scope)
        .into_iter()
        .flatten()
        .map(|affordance_id| {
            state
                .affordance_catalog
                .get(affordance_id)
                .map(|entry| (*affordance_id, entry))
                .ok_or_else(|| KernelError::Invariant("granted affordance has no entry".into()))
        })
        .collect()
}

/// One affordance the kernel has already gated: the opportunity offers this
/// entry, the scope holds it, and `entry` is its catalog definition. It is the
/// only way to name an affordance to `action::exercise`, so a caller cannot
/// reach the invocation pipeline without passing the grant check first.
pub(super) struct GrantedAffordance<'a> {
    pub(super) id: AffordanceId,
    pub(super) entry: &'a Affordance,
}

/// The membership half of the affordance check, shared by `reduce` and
/// `apply_effect`: the opportunity offers the entry and the scope holds it. It
/// returns the entry rather than a unit, so the gate and the lookup are one act.
fn require_granted<'a>(
    state: &'a WorldState,
    current: &DecisionOpportunity,
    affordance: AffordanceId,
) -> Result<GrantedAffordance<'a>, KernelError> {
    if current.affordance_ids.contains(&affordance)
        && state
            .affordance_grants
            .get(&current.scope)
            .is_some_and(|granted| granted.contains(&affordance))
    {
        let entry = state
            .affordance_catalog
            .get(&affordance)
            .ok_or_else(|| KernelError::Invariant("granted affordance has no entry".into()))?;
        Ok(GrantedAffordance {
            id: affordance,
            entry,
        })
    } else {
        Err(KernelError::AffordanceDenied)
    }
}

/// The sole producer of a `ScopeDigest`. The components it reads come from
/// `scope_components`, which the snapshot reads too.
fn scope_digest(state: &WorldState, scope: DecisionScope) -> Result<ScopeDigest, KernelError> {
    let controller = state
        .controller_assignments
        .get(&scope)
        .ok_or_else(|| KernelError::Invariant("decision scope has no controller".into()))?;
    let affordances = granted_entries(state, scope)?;
    let components = scope_components(state, scope.subject_id);
    digest(&ScopePreimage {
        world_id: state.world_id,
        subject_id: scope.subject_id,
        controller,
        affordances,
        components: &components,
    })
    .map(ScopeDigest)
}

/// Binding, which is one of the two checks a committed invocation passes.
/// Binding is scope-digest equality: has what the proposal was made against
/// moved? Admission — preconditions, effect ceilings, and the band draw, all in
/// `action::exercise` — is the other, and it runs at commit against the same
/// revision the digest was verified at. A component that admission reads but
/// binding does not is therefore fail-closed by re-check at commit rather than
/// by rejecting the proposal when it changes.
fn exact_opportunity(
    state: &WorldState,
    claimed: &DecisionOpportunity,
) -> Result<DecisionOpportunity, KernelError> {
    if claimed.world_id != state.world_id {
        return Err(KernelError::OpportunityMismatch);
    }
    let current = derive_opportunities(state)?
        .into_iter()
        .find(|current| current.scope == claimed.scope)
        .ok_or(KernelError::OpportunityMismatch)?;
    if claimed.revision > state.revision {
        return Err(KernelError::OpportunityMismatch);
    }
    if current.scope_digest != claimed.scope_digest {
        return Err(KernelError::ScopeChanged {
            scope: claimed.scope,
            expected: claimed.scope_digest.clone(),
            actual: current.scope_digest,
        });
    }
    Ok(current)
}

fn required_approvers(state: &WorldState) -> BTreeSet<PrincipalId> {
    std::iter::once(state.owner.clone())
        .chain(
            state
                .controller_assignments
                .values()
                .filter_map(ControllerAssignment::human_principal)
                .cloned(),
        )
        .collect()
}

/// Who may admit a patch, and inside what. `None` means unconfined: the owner.
/// Reached from `reduce`'s `AdmitPatch` arm and re-reached from `apply_effect`'s
/// `PatchAdmitted` arm, per the file's idiom that `reduce` decides and
/// `apply_effect` re-decides.
fn require_patch_author(
    state: &WorldState,
    caller: &CallerId,
    answers: Option<&PatchAnswer>,
) -> Result<Option<JurisdictionKey>, KernelError> {
    match caller {
        CallerId::Principal(principal) if principal == &state.owner => Ok(None),
        CallerId::System(SystemCapability::Elaborator { jurisdiction }) => {
            let answer = answers.ok_or(KernelError::Unauthorized)?;
            if !jurisdiction_covers(state, *jurisdiction, answer) {
                return Err(KernelError::Unauthorized);
            }
            Ok(Some(*jurisdiction))
        }
        _ => Err(KernelError::Unauthorized),
    }
}

/// Does the caller's jurisdiction cover the thing the answer names?
///
/// Two coverings, deliberately asymmetric. A boundary is covered
/// **transitively** through `covers_place`, so a parent jurisdiction's
/// elaborator may answer a boundary in a nested child. A deficit is covered by
/// **exact key equality**: a subject under nested roots counts toward both
/// roots' targets, so a parent answering a child's row would reduce two targets
/// with one subject and drain a child's queue while its own row stays red.
/// Boundaries name a thing; deficits name a row.
fn jurisdiction_covers(state: &WorldState, held: JurisdictionKey, answer: &PatchAnswer) -> bool {
    match answer {
        PatchAnswer::Boundary(CausalBoundary::UnelaboratedDestination { place, .. }) => {
            place_in(state, held, *place)
        }
        // The two never-derived variants join this clause only so the match is
        // total; `exact_boundary` refuses them before authority is read.
        PatchAnswer::Boundary(
            CausalBoundary::MissingStructure { subject, .. }
            | CausalBoundary::PolityInCausalRange { subject, .. }
            | CausalBoundary::IndividuationRequired {
                population: subject,
                ..
            },
        ) => match state.positions.get(subject) {
            Some(position) => place_in(state, held, position.place),
            None => held == JurisdictionKey::Uncovered,
        },
        PatchAnswer::Deficit(key) => *key == held,
    }
}

fn place_in(state: &WorldState, held: JurisdictionKey, place: EntityId) -> bool {
    match held {
        JurisdictionKey::PlaceSubtree(root) => patch::covers_place(&state.entities, root, place),
        JurisdictionKey::Uncovered => false,
    }
}

/// The ground a patch writes on, as one map over `state ∪ patch`, so a place
/// declared and built on in the same patch confines through the chain it
/// declares.
fn candidate_places(
    state: &WorldState,
    resolved: &ResolvedPatch,
) -> BTreeMap<EntityId, EntityRecord> {
    let mut entities = state.entities.clone();
    for declared in &resolved.entities {
        entities.insert(declared.entity_id, declared.entity.clone());
    }
    for declared in &resolved.facts {
        entities.insert(declared.entity_id, declared.entity.clone());
    }
    for declared in &resolved.channels {
        entities.insert(declared.entity_id, declared.entity.clone());
    }
    entities
}

/// Every place-or-subject referent a jurisdictional author writes must sit under
/// its jurisdiction. Returns the complete set, like every other check.
///
/// Placeless referents — resource kinds, catalog entries, canonical facts —
/// name no ground and are not confined. A place declared with no container is a
/// new jurisdiction root and is the owner's act. There is no relevance test: a
/// patch that satisfies its answer may also declare unrelated structure inside
/// its jurisdiction, because "the minimum the boundary needs" is a semantic
/// verdict the kernel must not hold.
fn confine_to_jurisdiction(
    state: &WorldState,
    resolved: &ResolvedPatch,
    held: JurisdictionKey,
) -> Result<(), Vec<Mismatch>> {
    let entities = candidate_places(state, resolved);
    let inside = |place: EntityId| match held {
        JurisdictionKey::PlaceSubtree(root) => patch::covers_place(&entities, root, place),
        JurisdictionKey::Uncovered => false,
    };
    let mut positions: BTreeMap<SubjectId, Option<EntityId>> = state
        .positions
        .iter()
        .map(|(subject, position)| (*subject, Some(position.place)))
        .collect();
    for declared in &resolved.subjects {
        positions.insert(
            declared.subject_id,
            declared.position.map(|position| position.place),
        );
    }
    let mut edges: BTreeMap<EdgeId, (EntityId, EntityId)> = state
        .edges
        .iter()
        .map(|(edge_id, record)| (*edge_id, record.endpoints()))
        .collect();
    for declared in &resolved.routes {
        edges.insert(declared.edge_id, declared.edge.endpoints());
    }

    let mut mismatches = Vec::new();
    let mut confine_subject = |site: &Site, subject: SubjectId, mismatches: &mut Vec<Mismatch>| {
        let admitted = match positions.get(&subject).copied().flatten() {
            Some(place) => inside(place),
            None => held == JurisdictionKey::Uncovered,
        };
        if !admitted {
            mismatches.push(Mismatch::OutsideJurisdiction { site: site.clone() });
        }
    };
    let mut confine_place = |site: &Site, place: EntityId, mismatches: &mut Vec<Mismatch>| {
        if !inside(place) {
            mismatches.push(Mismatch::OutsideJurisdiction { site: site.clone() });
        }
    };

    for declared in &resolved.subjects {
        let site = Site::Declaration(declared.handle.clone());
        confine_subject(&site, declared.subject_id, &mut mismatches);
    }
    for declared in &resolved.entities {
        if declared.entity.kind != EntityKind::Place {
            continue;
        }
        let site = Site::Declaration(declared.handle.clone());
        match declared.entity.container {
            Some(container) => confine_place(&site, container, &mut mismatches),
            // A root is the owner's act. An elaborator elaborates inside one.
            None => mismatches.push(Mismatch::OutsideJurisdiction { site }),
        }
    }
    for declared in &resolved.routes {
        let site = Site::Declaration(declared.handle.clone());
        let (from, to) = declared.edge.endpoints();
        confine_place(&site, from, &mut mismatches);
        confine_place(&site, to, &mut mismatches);
    }
    for declared in &resolved.facts {
        if let FactStanding::Claimed { by } = declared.fact.standing {
            let site = Site::Declaration(declared.handle.clone());
            confine_subject(&site, by, &mut mismatches);
        }
    }
    for declared in &resolved.channels {
        let site = Site::Declaration(declared.handle.clone());
        match &declared.channel.reach {
            Reach::Place(place) => confine_place(&site, *place, &mut mismatches),
            Reach::Subjects(subjects) => {
                for subject in subjects {
                    confine_subject(&site, *subject, &mut mismatches);
                }
            }
        }
        if let Some(controller) = declared.channel.controller {
            confine_subject(&site, controller, &mut mismatches);
        }
    }
    for (index, operation) in resolved.operations.iter().enumerate() {
        let site = Site::Operation(index);
        let (subjects, places, routes) = operation_ground(operation, &entities);
        for subject in subjects {
            confine_subject(&site, subject, &mut mismatches);
        }
        for place in places {
            confine_place(&site, place, &mut mismatches);
        }
        for route in routes {
            match edges.get(&route) {
                Some((from, to)) => {
                    confine_place(&site, *from, &mut mismatches);
                    confine_place(&site, *to, &mut mismatches);
                }
                None => mismatches.push(Mismatch::OutsideJurisdiction { site: site.clone() }),
            }
        }
    }
    mismatches.sort();
    mismatches.dedup();
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(mismatches)
    }
}

/// The ground one lowered operation touches: the subjects whose position it
/// must sit under, the places it names directly, and the routes whose endpoints
/// it must sit under. Total over `ResolvedOp`, so a new operation cannot ship
/// without saying where it lands.
fn operation_ground(
    operation: &ResolvedOp,
    entities: &BTreeMap<EntityId, EntityRecord>,
) -> (Vec<SubjectId>, Vec<EntityId>, Vec<EdgeId>) {
    // A resource, a fact, and a channel are placeless referents: they name no
    // ground, so only a place among the named entities confines.
    let places = |candidates: Vec<EntityId>| {
        candidates
            .into_iter()
            .filter(|entity_id| {
                entities
                    .get(entity_id)
                    .is_some_and(|record| record.kind == EntityKind::Place)
            })
            .collect()
    };
    match operation {
        ResolvedOp::Relocate {
            subject_id,
            edge_id,
        } => (vec![*subject_id], Vec::new(), vec![*edge_id]),
        ResolvedOp::OpenRoute { edge_id } | ResolvedOp::CloseRoute { edge_id } => {
            (Vec::new(), Vec::new(), vec![*edge_id])
        }
        ResolvedOp::AlterCost { edge_id, .. } => (Vec::new(), Vec::new(), vec![*edge_id]),
        ResolvedOp::Transfer {
            from, to, resource, ..
        } => (vec![*from, *to], places(vec![*resource]), Vec::new()),
        ResolvedOp::Transform {
            holder,
            from_resource,
            into_resource,
            ..
        } => (
            vec![*holder],
            places(vec![*from_resource, *into_resource]),
            Vec::new(),
        ),
        ResolvedOp::Consume {
            holder, resource, ..
        }
        | ResolvedOp::Admit {
            holder, resource, ..
        } => (vec![*holder], places(vec![*resource]), Vec::new()),
        ResolvedOp::Bind { subject, target } | ResolvedOp::Release { subject, target } => {
            match target {
                DependencyTarget::Subject(other) => {
                    (vec![*subject, *other], Vec::new(), Vec::new())
                }
                DependencyTarget::Route(edge_id) => (vec![*subject], Vec::new(), vec![*edge_id]),
                DependencyTarget::Resource(resource) => {
                    (vec![*subject], places(vec![*resource]), Vec::new())
                }
            }
        }
        ResolvedOp::GrantAuthority { holder, grant }
        | ResolvedOp::RevokeAuthority { holder, grant } => match grant.over {
            AuthorityTarget::Subject(other) => (vec![*holder, other], Vec::new(), Vec::new()),
            AuthorityTarget::PlaceSubtree(root) => (vec![*holder], vec![root], Vec::new()),
        },
        ResolvedOp::OpenOffice { institution, .. }
        | ResolvedOp::CloseOffice { institution, .. }
        | ResolvedOp::VacateOffice { institution, .. } => {
            (vec![*institution], Vec::new(), Vec::new())
        }
        ResolvedOp::InstallIncumbent {
            institution,
            incumbent,
            ..
        } => (vec![*institution, *incumbent], Vec::new(), Vec::new()),
        ResolvedOp::OpenForum {
            forum, standing, ..
        } => match standing {
            AuthorityTarget::Subject(other) => (vec![*forum, *other], Vec::new(), Vec::new()),
            AuthorityTarget::PlaceSubtree(root) => (vec![*forum], vec![*root], Vec::new()),
        },
        ResolvedOp::CloseForum { .. } => (Vec::new(), Vec::new(), Vec::new()),
        ResolvedOp::AcquireKnowledge { subject, .. } | ResolvedOp::Forget { subject, .. } => {
            (vec![*subject], Vec::new(), Vec::new())
        }
        ResolvedOp::Communicate { speaker, .. } => (vec![*speaker], Vec::new(), Vec::new()),
        ResolvedOp::SetReach { channel, reach } => {
            let mut subjects = Vec::new();
            let mut named = Vec::new();
            match reach {
                Reach::Place(place) => named.push(*place),
                Reach::Subjects(members) => subjects.extend(members.iter().copied()),
            }
            named.push(*channel);
            (subjects, places(named), Vec::new())
        }
        ResolvedOp::SetController {
            channel,
            controller,
        } => (
            controller.iter().copied().collect(),
            places(vec![*channel]),
            Vec::new(),
        ),
        // Kernel-synthesized by `action::exercise` and by nothing a patch may
        // carry; it is confined by its speaker all the same.
        ResolvedOp::AssertClaim { by, .. } => (vec![*by], Vec::new(), Vec::new()),
        ResolvedOp::CreateCommitment {
            subject,
            commitment,
            ..
        } => (
            std::iter::once(*subject)
                .chain(commitment.counterparty)
                .collect(),
            Vec::new(),
            Vec::new(),
        ),
        ResolvedOp::DischargeCommitment { subject, .. } => (vec![*subject], Vec::new(), Vec::new()),
        ResolvedOp::AdvancePressure { source, target, .. }
        | ResolvedOp::ReducePressure { source, target, .. }
        | ResolvedOp::ResolvePressure { source, target } => {
            let mut subjects = vec![*target];
            let mut named = Vec::new();
            let mut routes = Vec::new();
            match source {
                PressureSource::Commitment { subject, .. } | PressureSource::Subject(subject) => {
                    subjects.push(*subject);
                }
                PressureSource::Dependency(DependencyTarget::Subject(subject)) => {
                    subjects.push(*subject);
                }
                PressureSource::Dependency(DependencyTarget::Route(edge_id)) => {
                    routes.push(*edge_id);
                }
                PressureSource::Dependency(DependencyTarget::Resource(resource)) => {
                    named.push(*resource);
                }
            }
            (subjects, places(named), routes)
        }
    }
}

fn require_owner(state: &WorldState, caller: &CallerId) -> Result<(), KernelError> {
    if caller == &CallerId::Principal(state.owner.clone()) {
        Ok(())
    } else {
        Err(KernelError::Unauthorized)
    }
}

fn require_phase(state: &WorldState, expected: WorldPhase) -> Result<(), KernelError> {
    if state.phase == expected {
        Ok(())
    } else {
        Err(KernelError::WrongPhase {
            expected,
            actual: state.phase,
        })
    }
}

fn normalize_title(value: &str) -> Result<String, KernelError> {
    let value = value.trim();
    if value.is_empty() {
        Err(KernelError::EmptyTitle)
    } else {
        Ok(value.to_owned())
    }
}

fn validate_principal(value: &PrincipalId) -> Result<(), KernelError> {
    if value.0.trim().is_empty() || value.0.trim() != value.0 {
        Err(KernelError::EmptyPrincipal)
    } else {
        Ok(())
    }
}

fn validate_assignment(value: &ControllerAssignment) -> Result<(), KernelError> {
    if let ControllerAssignment::Human { principal, .. } = value {
        validate_principal(principal)?;
    }
    Ok(())
}

/// `command_id` is the band draw's only per-command term, so the arm that
/// re-derives an exercised decision needs the same one `reduce` drew from.
fn apply_effect(
    state: &mut WorldState,
    command_id: CommandId,
    caller: &CallerId,
    effect: &WorldEffect,
) -> Result<(), KernelError> {
    // The capability rule `reduce` decided, re-decided here against the effect
    // the command produced.
    if let CallerId::System(capability) = caller {
        let admitted = matches!(
            (capability, effect),
            (SystemCapability::Clock, WorldEffect::TimeAdvanced { .. })
                | (
                    SystemCapability::Elaborator { .. },
                    WorldEffect::PatchAdmitted { .. }
                )
        );
        if !admitted {
            return Err(KernelError::Unauthorized);
        }
    }
    match effect {
        WorldEffect::WorldCreated { .. } => {
            return Err(KernelError::Invariant(
                "world genesis cannot be applied as a mutable effect".into(),
            ));
        }
        WorldEffect::PatchAdmitted { answers, resolved } => {
            require_answer(state, answers.as_ref(), !resolved.declares_nothing())?;
            let confinement =
                require_patch_author(state, caller, answers.as_ref()).map_err(|_| {
                    KernelError::Invariant(
                        "admitted patch does not satisfy admission authority".into(),
                    )
                })?;
            if let Some(jurisdiction) = confinement {
                confine_to_jurisdiction(state, resolved, jurisdiction).map_err(|_| {
                    KernelError::Invariant(
                        "admitted patch wrote outside its author's jurisdiction".into(),
                    )
                })?;
            }
            // The deficit before the write, so "strictly decreased" is a
            // comparison rather than a claim the effect makes.
            let before = match answers {
                Some(PatchAnswer::Deficit(jurisdiction)) => {
                    jurisdiction_deficit(state, *jurisdiction)?
                }
                _ => 0,
            };
            admit_resolved(state, resolved)?;
            // What makes "a commit clears exactly the boundary it answers"
            // structural rather than hoped for. "Nothing else clears one" is
            // true by construction: boundaries are derived and no writer
            // clears one.
            match answers {
                Some(PatchAnswer::Boundary(answered)) => {
                    if exact_boundary(state, answered).is_ok() {
                        return Err(KernelError::AnswerNotSatisfied);
                    }
                }
                Some(PatchAnswer::Deficit(jurisdiction)) => {
                    if jurisdiction_deficit(state, *jurisdiction)? >= before {
                        return Err(KernelError::AnswerNotSatisfied);
                    }
                }
                None => {}
            }
        }
        WorldEffect::DraftApproved { principal } => {
            if state.phase != WorldPhase::Draft
                || caller != &CallerId::Principal(principal.clone())
                || !required_approvers(state).contains(principal)
                || !state.draft_approvals.insert(principal.clone())
            {
                return Err(KernelError::Invariant(
                    "draft-approval effect is unauthorized or duplicate".into(),
                ));
            }
        }
        WorldEffect::WorldActivated => {
            if caller != &CallerId::Principal(state.owner.clone())
                || state.phase != WorldPhase::Draft
                || !required_approvers(state).is_subset(&state.draft_approvals)
            {
                return Err(KernelError::Invariant(
                    "activation does not satisfy lifecycle authority".into(),
                ));
            }
            state.phase = WorldPhase::Active;
        }
        WorldEffect::DecisionExercised {
            opportunity,
            invocation,
            event,
        } => {
            let current = exact_opportunity(state, opportunity)?;
            let assignment = state
                .controller_assignments
                .get(&current.scope)
                .ok_or_else(|| {
                    KernelError::Invariant("decision scope lost its controller".into())
                })?;
            if state.phase != WorldPhase::Active || caller != &assignment.expected_caller() {
                return Err(KernelError::Invariant(
                    "decision effect does not match exact opportunity authority".into(),
                ));
            }
            let granted = require_granted(state, &current, invocation.affordance)?;
            // The whole event is re-derived by the function that produced the
            // honest one, so a forged band, operation, magnitude, minted claim,
            // or utterance is one comparison rather than a clause apiece.
            let derived = action::exercise(state, command_id, &current, &granted, invocation)?;
            if derived != *event {
                return Err(KernelError::Invariant(
                    "decision effect does not derive from its opportunity".into(),
                ));
            }
            state.events.push(event.clone());
            if !event.effects.is_empty() {
                apply_operations(state, &event.effects, &[])?;
            }
            state
                .last_opportunity_at
                .insert(current.scope.subject_id, state.now);
        }
        WorldEffect::DecisionDeclined { opportunity } => {
            let current = exact_opportunity(state, opportunity)?;
            let assignment = state
                .controller_assignments
                .get(&current.scope)
                .ok_or_else(|| {
                    KernelError::Invariant("decision scope lost its controller".into())
                })?;
            if state.phase != WorldPhase::Active || caller != &assignment.expected_caller() {
                return Err(KernelError::Invariant(
                    "decline effect does not match exact opportunity authority".into(),
                ));
            }
            // A decline consumed the turn: a controller that keeps declining
            // must not keep the head of the queue.
            state
                .last_opportunity_at
                .insert(current.scope.subject_id, state.now);
        }
        WorldEffect::TimeAdvanced {
            minutes,
            to,
            motion,
        } => {
            require_phase(state, WorldPhase::Active)?;
            require_clock_caller(state, caller)?;
            let derived_to = state
                .now
                .checked_add(*minutes)
                .ok_or_else(|| KernelError::Serialization("world clock overflow".into()))?;
            // The whole motion is re-derived rather than trusted field by
            // field, so a forged fulfilment, magnitude, row, and target are one
            // comparison.
            if derived_to != *to || clock::derive_motion(state, derived_to) != *motion {
                return Err(KernelError::Invariant(
                    "time effect does not derive from the world clock".into(),
                ));
            }
            state.now = *to;
            for rolled in &motion.fulfilled {
                let commitment = state
                    .commitments
                    .get_mut(&rolled.subject)
                    .and_then(|held| held.get_mut(&rolled.key))
                    .ok_or_else(|| {
                        KernelError::Invariant("a fulfilled routine has no commitment".into())
                    })?;
                commitment.due = rolled.next_due;
            }
            // No inner map is ever left empty, because `derive_motion` emits no
            // zero magnitude.
            for written in &motion.pressed {
                state
                    .pressures
                    .entry(written.target)
                    .or_default()
                    .insert(written.source, written.magnitude);
            }
        }
    }
    Ok(())
}

fn digest<T: Serialize>(value: &T) -> Result<String, KernelError> {
    let bytes = rmp_serde::to_vec_named(value)
        .map_err(|error| KernelError::Serialization(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn state_digest(state: &WorldState) -> Result<String, KernelError> {
    let mut value = state.clone();
    value.state_digest.clear();
    value.last_commit_digest = None;
    digest(&value)
}

fn commit_digest(commit: &WorldCommit) -> Result<String, KernelError> {
    let mut value = commit.clone();
    value.digest.clear();
    digest(&value)
}

#[cfg(test)]
mod tests {
    use super::patch::kernel_speak_grant;
    use super::*;

    /// What a committed event actually asserted, read from the one readable
    /// home of an utterance: `facts[fact].statement`. The committed event's
    /// own `AssertClaim` effect carries the same bytes as the replay witness,
    /// but no projection reads that copy; `facts` is the surface.
    pub(super) fn spoken<'a>(state: &'a WorldState, event: &DecisionEvent) -> Option<&'a str> {
        event
            .speech
            .and_then(|fact| state.facts.get(&fact))
            .map(|record| record.statement.as_str())
    }

    pub(super) fn owner() -> PrincipalId {
        PrincipalId::new("owner@example.test")
    }

    pub(super) fn player() -> PrincipalId {
        PrincipalId::new("player@example.test")
    }

    pub(super) fn auth_principal(principal: PrincipalId) -> AuthenticatedCaller {
        AuthenticatedCaller::fixture(CallerId::Principal(principal))
    }

    pub(super) fn subject(
        handle: &str,
        label: &str,
        kind: SubjectKind,
        controller: NewController,
    ) -> Declaration {
        Declaration::Subject(SubjectDeclaration {
            handle: DraftHandle::new(handle),
            label: label.into(),
            kind,
            controller,
            affordances: kernel_speak_grant(),
            // The fixture world is one room. Co-located speech needs a place to
            // fill, so every fixture subject stands in the commons.
            position: Some(Ref::Draft(DraftHandle::new(COMMONS))),
        })
    }

    /// The one place the base fixture world declares.
    pub(super) const COMMONS: &str = "commons";

    /// How many entities the base fixture world admits at genesis: the commons
    /// and nothing else. A patch-lane test that proves a rejection allocated
    /// nothing compares against this rather than against zero.
    pub(super) const FIXTURE_ENTITIES: usize = 1;

    /// The committed Speak entry. A post-genesis declaration grants it by
    /// canonical reference, like every other structure a previous commit
    /// allocated: the kernel synthesizes the entry once, at genesis.
    pub(super) fn speak_entry(kernel: &WorldKernel) -> Ref<AffordanceId> {
        Ref::Existing(
            *kernel
                .state
                .affordance_catalog
                .iter()
                .find(|(_, entry)| entry.kind.0 == "speak")
                .map(|(affordance_id, _)| affordance_id)
                .expect("genesis admits the kernel Speak entry"),
        )
    }

    /// A catalog entry by kind name. An affordance id names an entry, not a
    /// subject's copy of one, so a test that needs an entry a scope does not
    /// hold asks for it by name rather than by list position.
    pub(super) fn affordance_named(snapshot: &WorldSnapshot, kind: &str) -> AffordanceId {
        snapshot
            .affordances
            .iter()
            .find(|entry| entry.entry.kind.0 == kind)
            .map(|entry| entry.id)
            .expect("the fixture world declares this affordance")
    }

    pub(super) fn creation(id: CommandId, title: &str) -> CreateWorld {
        CreateWorld {
            id,
            owner: owner(),
            title: title.into(),
            patch: WorldPatch {
                operations: Vec::new(),
                evidence: Vec::new(),
                declarations: vec![
                    Declaration::Entity(EntityDeclaration {
                        handle: DraftHandle::new(COMMONS),
                        label: "The Commons".into(),
                        kind: EntityKind::Place,
                        container: None,
                    }),
                    // A second, world-declared entry granted to exactly one
                    // subject, so the fixture world has more than one verb and
                    // grant sets differ between scopes.
                    Declaration::Affordance(AffordanceDeclaration {
                        handle: DraftHandle::new("convene"),
                        kind: AffordanceKindName("convene".into()),
                        roles: Vec::new(),
                        // A speech-carrying entry must name exactly one
                        // audience for the lowering to read.
                        preconditions: vec![Precondition::CanBroadcast {
                            via: AudienceSpec::Colocated,
                        }],
                        effect_slots: Vec::new(),
                        outcome_bands: vec![OutcomeBand {
                            weight: 1,
                            effects: Vec::new(),
                        }],
                        carries_speech: true,
                    }),
                    subject(
                        "player",
                        "The Player",
                        SubjectKind::Person,
                        NewController::Human {
                            principal: player(),
                        },
                    ),
                    subject(
                        "persona",
                        "The Witness",
                        SubjectKind::Person,
                        NewController::NarrativePersona,
                    ),
                    Declaration::Subject(SubjectDeclaration {
                        handle: DraftHandle::new("operator"),
                        label: "The Council".into(),
                        kind: SubjectKind::Institution,
                        controller: NewController::OperationalAgent,
                        affordances: kernel_speak_grant()
                            .into_iter()
                            .chain(std::iter::once(Ref::Draft(DraftHandle::new("convene"))))
                            .collect(),
                        position: Some(Ref::Draft(DraftHandle::new(COMMONS))),
                    }),
                ],
            },
            scale_intent: WorldScaleIntentRef::default(),
        }
    }

    pub(super) fn command(
        snapshot: &WorldSnapshot,
        id: CommandId,
        caller: CallerId,
        body: CommandBody,
    ) -> CommandEnvelope {
        CommandEnvelope {
            id,
            world_id: snapshot.world_id,
            expected_revision: snapshot.revision,
            caller,
            body,
        }
    }

    pub(super) fn submit_owner(
        kernel: &mut WorldKernel,
        snapshot: &WorldSnapshot,
        body: CommandBody,
    ) -> SubmitReceipt {
        kernel
            .submit(
                command(
                    snapshot,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    body,
                ),
                &auth_principal(owner()),
            )
            .unwrap()
    }

    pub(super) fn activate(kernel: &mut WorldKernel) -> WorldSnapshot {
        let genesis = kernel.snapshot().unwrap();
        submit_owner(kernel, &genesis, CommandBody::ApproveDraft);
        let after_owner = kernel.snapshot().unwrap();
        kernel
            .submit(
                command(
                    &after_owner,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ApproveDraft,
                ),
                &auth_principal(player()),
            )
            .unwrap();
        let approved = kernel.snapshot().unwrap();
        submit_owner(kernel, &approved, CommandBody::ActivateWorld);
        kernel.snapshot().unwrap()
    }

    /// The canonical IDs of the shared topology fixture: three places, four
    /// routes with every access and open combination the operations read, and one
    /// subject standing in the yard.
    pub(super) struct Topology {
        pub(super) yard: EntityId,
        pub(super) road: EntityId,
        pub(super) gate: EntityId,
        pub(super) ramp: EdgeId,
        pub(super) shutter: EdgeId,
        pub(super) toll: EdgeId,
        pub(super) span: EdgeId,
        pub(super) walker: SubjectId,
    }

    fn place(handle: &str, label: &str) -> Declaration {
        Declaration::Entity(EntityDeclaration {
            handle: DraftHandle::new(handle),
            label: label.into(),
            kind: EntityKind::Place,
            container: None,
        })
    }

    fn way(
        handle: &str,
        label: &str,
        from: &str,
        to: &str,
        access: AccessKind,
        cost: u32,
    ) -> Declaration {
        Declaration::Route(RouteDeclaration {
            handle: DraftHandle::new(handle),
            label: label.into(),
            from: Ref::Draft(DraftHandle::new(from)),
            to: Ref::Draft(DraftHandle::new(to)),
            access,
            cost: Cost(cost),
        })
    }

    pub(super) fn topology_patch(speak: Ref<AffordanceId>) -> WorldPatch {
        WorldPatch {
            declarations: vec![
                place("yard", "The Cavity Yard"),
                place("road", "The Rhythm Road"),
                place("gate", "The Rain Gate"),
                way(
                    "ramp",
                    "The Yard Ramp",
                    "yard",
                    "road",
                    AccessKind::Public,
                    12,
                ),
                way(
                    "shutter",
                    "The Yard Shutter",
                    "yard",
                    "gate",
                    AccessKind::Public,
                    5,
                ),
                way(
                    "toll",
                    "The Toll Stair",
                    "yard",
                    "gate",
                    AccessKind::Restricted {
                        requires: AuthorityKindName(ADMIT_KIND.into()),
                    },
                    4,
                ),
                way(
                    "span",
                    "The Western Span",
                    "road",
                    "gate",
                    AccessKind::Public,
                    7,
                ),
                Declaration::Subject(SubjectDeclaration {
                    handle: DraftHandle::new("walker"),
                    label: "The Walker".into(),
                    kind: SubjectKind::Person,
                    controller: NewController::OperationalAgent,
                    affordances: BTreeSet::from([speak]),
                    position: Some(Ref::Draft(DraftHandle::new("yard"))),
                }),
            ],
            // A route that should start closed is declared and closed in one
            // patch, so `open` keeps a single writer family.
            operations: vec![ComponentOp::CloseRoute {
                route: Ref::Draft(DraftHandle::new("shutter")),
            }],
            evidence: Vec::new(),
        }
    }

    pub(super) fn admit_topology(kernel: &mut WorldKernel) -> Topology {
        let before = kernel.snapshot().unwrap();
        let receipt = submit_owner(
            kernel,
            &before,
            CommandBody::AdmitPatch {
                answers: None,
                patch: topology_patch(speak_entry(kernel)),
            },
        );
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
        let entity = |label: &str| {
            *kernel
                .state
                .entities
                .iter()
                .find(|(_, record)| record.label == label)
                .expect("a declared place")
                .0
        };
        let edge = |label: &str| {
            *kernel
                .state
                .edges
                .iter()
                .find(|(_, record)| record.label() == label)
                .expect("a declared route")
                .0
        };
        Topology {
            yard: entity("The Cavity Yard"),
            road: entity("The Rhythm Road"),
            gate: entity("The Rain Gate"),
            ramp: edge("The Yard Ramp"),
            shutter: edge("The Yard Shutter"),
            toll: edge("The Toll Stair"),
            span: edge("The Western Span"),
            walker: *kernel
                .state
                .subjects
                .iter()
                .find(|(_, subject)| subject.label == "The Walker")
                .expect("the walker is admitted")
                .0,
        }
    }

    /// The canonical IDs of the custody fixture: two resources and two holder
    /// subjects standing in the topology, with the first holder carrying an
    /// evidenced opening balance.
    pub(super) struct Custody {
        pub(super) tithe: EntityId,
        pub(super) ingot: EntityId,
        pub(super) holder: SubjectId,
        pub(super) counterparty: SubjectId,
    }

    /// The seed world's authority kinds, offices, and grievance. They are world
    /// data: the kernel compares these strings and reads them no other way.
    pub(super) const LEVY_KIND: &str = "levy";
    pub(super) const ADMIT_KIND: &str = "admit";
    pub(super) const COMMAND_KIND: &str = "command";
    pub(super) const JUDGE_KIND: &str = "judge";
    pub(super) const WARDEN_OFFICE: &str = "warden";
    pub(super) const BAILIFF_OFFICE: &str = "bailiff";
    pub(super) const SEIZURE_GRIEVANCE: &str = "seizure";

    pub(super) const TITHE_RECEIPT: &str = "receipt:rhythm-tithe-census";
    pub(super) const OPENING_BALANCE: u64 = 7;

    fn resource(handle: &str, label: &str) -> Declaration {
        Declaration::Entity(EntityDeclaration {
            handle: DraftHandle::new(handle),
            label: label.into(),
            kind: EntityKind::Resource,
            container: None,
        })
    }

    fn holder(handle: &str, label: &str, place: EntityId, speak: Ref<AffordanceId>) -> Declaration {
        Declaration::Subject(SubjectDeclaration {
            handle: DraftHandle::new(handle),
            label: label.into(),
            kind: SubjectKind::Institution,
            controller: NewController::OperationalAgent,
            affordances: std::iter::once(speak)
                .chain(carry_affordances().into_iter().map(|declaration| {
                    let Declaration::Affordance(entry) = declaration else {
                        unreachable!("carry_affordances declares only affordances")
                    };
                    Ref::Draft(entry.handle)
                }))
                .collect(),
            position: Some(Ref::Existing(place)),
        })
    }

    pub(super) const CARRY_HANDLE: &str = "carry";

    /// The worked affordances for the action lane. Every one carries the same
    /// four roles and the same bounded `Transfer` slot, so a test varies exactly
    /// one thing — a precondition or a band table — by naming a different entry
    /// rather than by reaching into a committed catalog. The catalog is
    /// Draft-only, so authoring it is how a world gets a verb.
    fn carry_roles() -> Vec<RoleSpec> {
        vec![
            RoleSpec {
                role: Role("from".into()),
                kind: RefKind::Subject(None),
            },
            RoleSpec {
                role: Role("recipient".into()),
                kind: RefKind::Subject(None),
            },
            RoleSpec {
                role: Role("place".into()),
                kind: RefKind::Entity(EntityKind::Place),
            },
            RoleSpec {
                role: Role("resource".into()),
                kind: RefKind::Entity(EntityKind::Resource),
            },
        ]
    }

    /// Two slots on disjoint bands: band 0 names only the transfer, band 1 only
    /// the consume. Both slots are proposed on every invocation, so the entry
    /// separates what a proposer offered from what a draw admitted.
    fn split_variant() -> Declaration {
        Declaration::Affordance(AffordanceDeclaration {
            handle: DraftHandle::new("carry-split"),
            kind: AffordanceKindName("carry_split".into()),
            roles: carry_roles(),
            preconditions: Vec::new(),
            effect_slots: vec![
                EffectSlot {
                    op_kind: ComponentOpKind::Transfer,
                    roles: vec![
                        Role("from".into()),
                        Role("recipient".into()),
                        Role("resource".into()),
                    ],
                    bounds: Bounds::Quantity(Quantity(3)),
                },
                EffectSlot {
                    op_kind: ComponentOpKind::Consume,
                    roles: vec![Role("from".into()), Role("resource".into())],
                    bounds: Bounds::Quantity(Quantity(3)),
                },
            ],
            outcome_bands: vec![
                OutcomeBand {
                    weight: 1,
                    effects: vec![0],
                },
                OutcomeBand {
                    weight: 1,
                    effects: vec![1],
                },
            ],
            carries_speech: false,
        })
    }

    fn carry_variant(
        handle: &str,
        kind: &str,
        preconditions: Vec<Precondition>,
        outcome_bands: Vec<OutcomeBand>,
    ) -> Declaration {
        Declaration::Affordance(AffordanceDeclaration {
            handle: DraftHandle::new(handle),
            kind: AffordanceKindName(kind.into()),
            roles: carry_roles(),
            preconditions,
            effect_slots: vec![EffectSlot {
                op_kind: ComponentOpKind::Transfer,
                roles: vec![
                    Role("from".into()),
                    Role("recipient".into()),
                    Role("resource".into()),
                ],
                bounds: Bounds::Quantity(Quantity(3)),
            }],
            outcome_bands,
            carries_speech: false,
        })
    }

    fn certain_band() -> Vec<OutcomeBand> {
        vec![OutcomeBand {
            weight: 1,
            effects: vec![0],
        }]
    }

    fn place_role() -> Role {
        Role("place".into())
    }

    /// Carry, and five variants that each move one dial: a holding demand above
    /// the opening balance, a reach budget short of the two-hop path, one that
    /// covers it, a band that names no effect, and three equally weighted bands.
    pub(super) fn carry_affordances() -> Vec<Declaration> {
        vec![
            carry_variant(
                CARRY_HANDLE,
                "carry",
                vec![
                    Precondition::Present { at: place_role() },
                    Precondition::Holds {
                        resource: Role("resource".into()),
                        at_least: Quantity(1),
                    },
                ],
                certain_band(),
            ),
            carry_variant(
                "carry-greedy",
                "carry_greedy",
                vec![
                    Precondition::Present { at: place_role() },
                    Precondition::Holds {
                        resource: Role("resource".into()),
                        at_least: Quantity(OPENING_BALANCE + 1),
                    },
                ],
                certain_band(),
            ),
            carry_variant(
                "carry-holds",
                "carry_holds",
                vec![Precondition::Holds {
                    resource: Role("resource".into()),
                    at_least: Quantity(1),
                }],
                certain_band(),
            ),
            carry_variant(
                "carry-near",
                "carry_near",
                vec![Precondition::Reachable {
                    to: place_role(),
                    within: Cost(NEAR_REACH),
                }],
                certain_band(),
            ),
            carry_variant(
                "carry-far",
                "carry_far",
                vec![Precondition::Reachable {
                    to: place_role(),
                    within: Cost(FAR_REACH),
                }],
                certain_band(),
            ),
            carry_variant(
                "carry-idle",
                "carry_idle",
                Vec::new(),
                vec![OutcomeBand {
                    weight: 1,
                    effects: Vec::new(),
                }],
            ),
            carry_variant(
                "carry-chance",
                "carry_chance",
                Vec::new(),
                vec![
                    OutcomeBand {
                        weight: 1,
                        effects: vec![0],
                    },
                    OutcomeBand {
                        weight: 1,
                        effects: Vec::new(),
                    },
                    OutcomeBand {
                        weight: 1,
                        effects: Vec::new(),
                    },
                ],
            ),
            split_variant(),
        ]
    }

    /// Yard to gate over the open shutter alone.
    pub(super) const NEAR_REACH: u32 = 5;
    /// Yard to gate over the ramp and the span, which is the only open public
    /// path while the shutter is closed.
    pub(super) const FAR_REACH: u32 = 19;

    /// Declarations and evidence are Draft-only, so the resources, the holders,
    /// and the one evidenced `Admit` that creates the opening balance all land
    /// before activation. There is no holdings declaration field: quantity is
    /// created by `Admit` and by nothing else, in this lane as in every other.
    pub(super) fn custody_patch(topology: &Topology, speak: Ref<AffordanceId>) -> WorldPatch {
        WorldPatch {
            declarations: carry_affordances()
                .into_iter()
                .chain([
                    resource("tithe", "The Rhythm Tithe"),
                    resource("ingot", "The Cut Ingot"),
                    holder("clerk", "The Ledger Clerk", topology.yard, speak.clone()),
                    holder("keeper", "The Gate Keeper", topology.gate, speak),
                ])
                .collect(),
            operations: vec![ComponentOp::Admit {
                holder: Ref::Draft(DraftHandle::new("clerk")),
                resource: Ref::Draft(DraftHandle::new("tithe")),
                qty: Quantity(OPENING_BALANCE),
                evidence: EvidenceRef::new(TITHE_RECEIPT),
            }],
            evidence: vec![EvidenceRef::new(TITHE_RECEIPT)],
        }
    }

    pub(super) fn admit_custody(kernel: &mut WorldKernel, topology: &Topology) -> Custody {
        let before = kernel.snapshot().unwrap();
        let receipt = submit_owner(
            kernel,
            &before,
            CommandBody::AdmitPatch {
                answers: None,
                patch: custody_patch(topology, speak_entry(kernel)),
            },
        );
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
        let entity = |label: &str| {
            *kernel
                .state
                .entities
                .iter()
                .find(|(_, record)| record.label == label)
                .expect("a declared resource")
                .0
        };
        let subject = |label: &str| {
            *kernel
                .state
                .subjects
                .iter()
                .find(|(_, record)| record.label == label)
                .expect("a declared holder")
                .0
        };
        Custody {
            tithe: entity("The Rhythm Tithe"),
            ingot: entity("The Cut Ingot"),
            holder: subject("The Ledger Clerk"),
            counterparty: subject("The Gate Keeper"),
        }
    }

    /// Topology, custody, then activation: everything a custody test needs, in
    /// the one order the phase rules allow.
    pub(super) fn custody_world(kernel: &mut WorldKernel) -> (Topology, Custody, WorldSnapshot) {
        let topology = admit_topology(kernel);
        let custody = admit_custody(kernel, &topology);
        let active = activate(kernel);
        (topology, custody, active)
    }

    /// The canonical IDs of the civic fixture: a hall containing a chamber, an
    /// institution holding jurisdiction over the hall, a person who holds that
    /// jurisdiction only through an office, a person and a resource inside the
    /// hall, and a person standing outside it.
    pub(super) struct Civic {
        pub(super) hall: EntityId,
        pub(super) chamber: EntityId,
        pub(super) passage: EdgeId,
        pub(super) causeway: EdgeId,
        pub(super) postern: EdgeId,
        pub(super) treasury: SubjectId,
        pub(super) reeve: SubjectId,
        pub(super) farmer: SubjectId,
        pub(super) outsider: SubjectId,
        pub(super) grain: EntityId,
    }

    pub(super) fn authority_kind(name: &str) -> AuthorityKindName {
        AuthorityKindName(name.into())
    }

    pub(super) fn office(name: &str) -> OfficeName {
        OfficeName(name.into())
    }

    pub(super) fn grievance(name: &str) -> GrievanceKindName {
        GrievanceKindName(name.into())
    }

    pub(super) fn over_place(place: EntityId) -> AuthorityTargetRef {
        AuthorityTargetRef::PlaceSubtree(Ref::Existing(place))
    }

    pub(super) fn over_subject(subject_id: SubjectId) -> AuthorityTargetRef {
        AuthorityTargetRef::Subject(Ref::Existing(subject_id))
    }

    pub(super) fn grant_of(kind: &str, over: AuthorityTargetRef) -> AuthorityGrantRef {
        AuthorityGrantRef {
            kind: authority_kind(kind),
            over,
        }
    }

    pub(super) fn grant_to(holder: SubjectId, kind: &str, over: AuthorityTargetRef) -> ComponentOp {
        ComponentOp::GrantAuthority {
            holder: Ref::Existing(holder),
            grant: grant_of(kind, over),
        }
    }

    /// Seven world-authored civic entries. The kernel builds none of them;
    /// each is a worked example of what a seed author writes to make the
    /// political layer playable rather than administratively imposed.
    fn civic_affordances() -> Vec<Declaration> {
        let entry = |handle: &str,
                     kind: &str,
                     roles: Vec<RoleSpec>,
                     preconditions: Vec<Precondition>,
                     effect_slots: Vec<EffectSlot>,
                     outcome_bands: Vec<OutcomeBand>,
                     carries_speech: bool| {
            Declaration::Affordance(AffordanceDeclaration {
                handle: DraftHandle::new(handle),
                kind: AffordanceKindName(kind.into()),
                roles,
                preconditions,
                effect_slots,
                outcome_bands,
                carries_speech,
            })
        };
        let role = |name: &str, kind: RefKind| RoleSpec {
            role: Role(name.into()),
            kind,
        };
        let slot = |op_kind: ComponentOpKind, roles: Vec<&str>, bounds: Bounds| EffectSlot {
            op_kind,
            roles: roles.into_iter().map(|name| Role(name.into())).collect(),
            bounds,
        };
        let authorized = |over: &str, kind: &str| Precondition::Authorized {
            over: Role(over.into()),
            kind: authority_kind(kind),
        };
        vec![
            // The payee is the reserved `actor` role, so an authorized
            // collector cannot lawfully take a tax and send it to a friend.
            entry(
                "levy",
                "levy",
                vec![
                    role("payer", RefKind::Subject(None)),
                    role("resource", RefKind::Entity(EntityKind::Resource)),
                ],
                vec![authorized("payer", LEVY_KIND)],
                vec![slot(
                    ComponentOpKind::Transfer,
                    vec!["payer", "actor", "resource"],
                    Bounds::Quantity(Quantity(10)),
                )],
                certain_band(),
                false,
            ),
            entry(
                "delegate",
                "delegate",
                vec![
                    role("deputy", RefKind::Subject(None)),
                    role("ground", RefKind::Entity(EntityKind::Place)),
                ],
                // The precondition names one kind and the slot grants another,
                // so the monotonicity rule is the check that decides whether
                // the granted authority is one the granter holds.
                vec![authorized("ground", COMMAND_KIND)],
                vec![slot(
                    ComponentOpKind::GrantAuthority {
                        kind: authority_kind(LEVY_KIND),
                    },
                    vec!["deputy", "ground"],
                    Bounds::None,
                )],
                certain_band(),
                false,
            ),
            // Isolates the action-lane's revocation envelope: no separate
            // precondition, so a rejection here can only be
            // `DelegationNotMonotone`.
            entry(
                "revoke",
                "revoke",
                vec![
                    role("holder", RefKind::Subject(None)),
                    role("ground", RefKind::Entity(EntityKind::Place)),
                ],
                Vec::new(),
                vec![slot(
                    ComponentOpKind::RevokeAuthority {
                        kind: authority_kind(LEVY_KIND),
                    },
                    vec!["holder", "ground"],
                    Bounds::None,
                )],
                certain_band(),
                false,
            ),
            entry(
                "deploy",
                "deploy",
                vec![
                    role("subordinate", RefKind::Subject(None)),
                    role("via", RefKind::Edge(patch::EdgeKind::Route)),
                ],
                vec![authorized("subordinate", COMMAND_KIND)],
                vec![slot(
                    ComponentOpKind::Relocate,
                    vec!["subordinate", "via"],
                    Bounds::None,
                )],
                certain_band(),
                false,
            ),
            // The first affordance whose effect changes shared topology under a
            // legitimacy check; the second band is the interdiction that fails.
            entry(
                "sanction",
                "sanction",
                vec![role("road", RefKind::Edge(patch::EdgeKind::Route))],
                vec![authorized("road", COMMAND_KIND)],
                vec![slot(
                    ComponentOpKind::CloseRoute,
                    vec!["road"],
                    Bounds::None,
                )],
                vec![
                    OutcomeBand {
                        weight: 3,
                        effects: vec![0],
                    },
                    OutcomeBand {
                        weight: 1,
                        effects: Vec::new(),
                    },
                ],
                false,
            ),
            // Succession is an act someone performs under authority, not a
            // declared method string.
            entry(
                "appoint",
                "appoint",
                vec![
                    role("institution", RefKind::Subject(None)),
                    role("candidate", RefKind::Subject(None)),
                ],
                vec![authorized("institution", COMMAND_KIND)],
                vec![slot(
                    ComponentOpKind::InstallIncumbent {
                        office: office(BAILIFF_OFFICE),
                    },
                    vec!["institution", "candidate"],
                    Bounds::None,
                )],
                certain_band(),
                false,
            ),
            // The reader that keeps `Redress` from being decorative.
            entry(
                "petition",
                "petition",
                Vec::new(),
                vec![
                    Precondition::HasStanding {
                        grievance: grievance(SEIZURE_GRIEVANCE),
                    },
                    // A petition is spoken, so the entry names where it lands.
                    Precondition::CanBroadcast {
                        via: AudienceSpec::Colocated,
                    },
                ],
                Vec::new(),
                vec![OutcomeBand {
                    weight: 1,
                    effects: Vec::new(),
                }],
                true,
            ),
        ]
    }

    fn civic_grants(handles: &[&str], speak: &Ref<AffordanceId>) -> BTreeSet<Ref<AffordanceId>> {
        std::iter::once(speak.clone())
            .chain(
                handles
                    .iter()
                    .map(|handle| Ref::Draft(DraftHandle::new(*handle))),
            )
            .collect()
    }

    pub(super) fn civic_patch(topology: &Topology, speak: Ref<AffordanceId>) -> WorldPatch {
        let person = |handle: &str, label: &str, place: Ref<EntityId>, handles: &[&str]| {
            Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new(handle),
                label: label.into(),
                kind: SubjectKind::Person,
                controller: NewController::OperationalAgent,
                affordances: civic_grants(handles, &speak),
                position: Some(place),
            })
        };
        let declarations = civic_affordances()
            .into_iter()
            .chain([
                Declaration::Entity(EntityDeclaration {
                    handle: DraftHandle::new("hall"),
                    label: "The Tithe Hall".into(),
                    kind: EntityKind::Place,
                    container: None,
                }),
                Declaration::Entity(EntityDeclaration {
                    handle: DraftHandle::new("chamber"),
                    label: "The Counting Chamber".into(),
                    kind: EntityKind::Place,
                    container: Some(Ref::Draft(DraftHandle::new("hall"))),
                }),
                resource("grain", "The Winter Grain"),
                Declaration::Route(RouteDeclaration {
                    handle: DraftHandle::new("passage"),
                    label: "The Hall Passage".into(),
                    from: Ref::Draft(DraftHandle::new("chamber")),
                    to: Ref::Draft(DraftHandle::new("hall")),
                    access: AccessKind::Public,
                    cost: Cost(3),
                }),
                Declaration::Route(RouteDeclaration {
                    handle: DraftHandle::new("causeway"),
                    label: "The Hall Causeway".into(),
                    from: Ref::Draft(DraftHandle::new("hall")),
                    to: Ref::Existing(topology.road),
                    access: AccessKind::Public,
                    cost: Cost(6),
                }),
                Declaration::Route(RouteDeclaration {
                    handle: DraftHandle::new("postern"),
                    label: "The Hall Postern".into(),
                    from: Ref::Existing(topology.yard),
                    to: Ref::Draft(DraftHandle::new("chamber")),
                    access: AccessKind::Restricted {
                        requires: authority_kind(ADMIT_KIND),
                    },
                    cost: Cost(2),
                }),
                Declaration::Subject(SubjectDeclaration {
                    handle: DraftHandle::new("treasury"),
                    label: "The Tithe Treasury".into(),
                    kind: SubjectKind::Institution,
                    controller: NewController::OperationalAgent,
                    affordances: civic_grants(
                        &[
                            "levy", "delegate", "deploy", "sanction", "appoint", "revoke",
                        ],
                        &speak,
                    ),
                    position: Some(Ref::Draft(DraftHandle::new("hall"))),
                }),
                Declaration::Subject(SubjectDeclaration {
                    handle: DraftHandle::new("reeve"),
                    label: "The Hall Reeve".into(),
                    kind: SubjectKind::Person,
                    controller: NewController::NarrativePersona,
                    affordances: civic_grants(
                        &["levy", "delegate", "petition", "sanction"],
                        &speak,
                    ),
                    position: Some(Ref::Draft(DraftHandle::new("chamber"))),
                }),
                person(
                    "farmer",
                    "The Winter Farmer",
                    Ref::Draft(DraftHandle::new("chamber")),
                    &["levy", "petition", "revoke"],
                ),
                person(
                    "outsider",
                    "The Road Pedlar",
                    Ref::Existing(topology.road),
                    &["levy", "petition"],
                ),
                // Deliberately unplaced: a place-subtree jurisdiction covers a
                // subject through its position, so a subject with none is
                // covered by nothing.
                Declaration::Subject(SubjectDeclaration {
                    handle: DraftHandle::new("nowhere"),
                    label: "The Unhoused".into(),
                    kind: SubjectKind::Person,
                    controller: NewController::NarrativePersona,
                    affordances: civic_grants(&["levy", "petition"], &speak),
                    position: None,
                }),
            ])
            .collect();
        WorldPatch {
            declarations,
            operations: vec![
                ComponentOp::Admit {
                    holder: Ref::Draft(DraftHandle::new("farmer")),
                    resource: Ref::Draft(DraftHandle::new("grain")),
                    qty: Quantity(OPENING_BALANCE),
                    evidence: EvidenceRef::new(TITHE_RECEIPT),
                },
                ComponentOp::Admit {
                    holder: Ref::Draft(DraftHandle::new("outsider")),
                    resource: Ref::Draft(DraftHandle::new("grain")),
                    qty: Quantity(OPENING_BALANCE),
                    evidence: EvidenceRef::new(TITHE_RECEIPT),
                },
                ComponentOp::GrantAuthority {
                    holder: Ref::Draft(DraftHandle::new("treasury")),
                    grant: AuthorityGrantRef {
                        kind: authority_kind(LEVY_KIND),
                        over: AuthorityTargetRef::PlaceSubtree(Ref::Draft(DraftHandle::new(
                            "hall",
                        ))),
                    },
                },
                ComponentOp::GrantAuthority {
                    holder: Ref::Draft(DraftHandle::new("treasury")),
                    grant: AuthorityGrantRef {
                        kind: authority_kind(COMMAND_KIND),
                        over: AuthorityTargetRef::PlaceSubtree(Ref::Draft(DraftHandle::new(
                            "hall",
                        ))),
                    },
                },
                ComponentOp::OpenOffice {
                    institution: Ref::Draft(DraftHandle::new("treasury")),
                    office: office(WARDEN_OFFICE),
                    delegated: BTreeSet::from([authority_kind(LEVY_KIND)]),
                },
                ComponentOp::OpenOffice {
                    institution: Ref::Draft(DraftHandle::new("treasury")),
                    office: office(BAILIFF_OFFICE),
                    delegated: BTreeSet::from([authority_kind(JUDGE_KIND)]),
                },
                ComponentOp::InstallIncumbent {
                    institution: Ref::Draft(DraftHandle::new("treasury")),
                    office: office(WARDEN_OFFICE),
                    incumbent: Ref::Draft(DraftHandle::new("reeve")),
                },
                ComponentOp::OpenForum {
                    grievance: grievance(SEIZURE_GRIEVANCE),
                    forum: Ref::Draft(DraftHandle::new("treasury")),
                    standing: AuthorityTargetRef::PlaceSubtree(Ref::Draft(DraftHandle::new(
                        "chamber",
                    ))),
                },
            ],
            evidence: vec![EvidenceRef::new(TITHE_RECEIPT)],
        }
    }

    pub(super) fn admit_civic(kernel: &mut WorldKernel, topology: &Topology) -> Civic {
        let before = kernel.snapshot().unwrap();
        let receipt = submit_owner(
            kernel,
            &before,
            CommandBody::AdmitPatch {
                answers: None,
                patch: civic_patch(topology, speak_entry(kernel)),
            },
        );
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
        let entity = |label: &str| {
            *kernel
                .state
                .entities
                .iter()
                .find(|(_, record)| record.label == label)
                .expect("a declared entity")
                .0
        };
        let edge = |label: &str| {
            *kernel
                .state
                .edges
                .iter()
                .find(|(_, record)| record.label() == label)
                .expect("a declared route")
                .0
        };
        let subject = |label: &str| {
            *kernel
                .state
                .subjects
                .iter()
                .find(|(_, record)| record.label == label)
                .expect("a declared subject")
                .0
        };
        Civic {
            hall: entity("The Tithe Hall"),
            chamber: entity("The Counting Chamber"),
            grain: entity("The Winter Grain"),
            passage: edge("The Hall Passage"),
            causeway: edge("The Hall Causeway"),
            postern: edge("The Hall Postern"),
            treasury: subject("The Tithe Treasury"),
            reeve: subject("The Hall Reeve"),
            farmer: subject("The Winter Farmer"),
            outsider: subject("The Road Pedlar"),
        }
    }

    /// Topology, the civic subgraph, then activation.
    pub(super) fn civic_world(kernel: &mut WorldKernel) -> (Topology, Civic, WorldSnapshot) {
        let topology = admit_topology(kernel);
        let civic = admit_civic(kernel, &topology);
        let active = activate(kernel);
        (topology, civic, active)
    }

    /// The speech bench: a hall containing a yard, a speaker and a listener in
    /// the hall, a bystander in the yard, a stranger nowhere, one evidenced
    /// canonical fact, and a horn that carries over the hall subtree.
    pub(super) struct Speech {
        pub(super) hall: EntityId,
        pub(super) yard: EntityId,
        pub(super) speaker: SubjectId,
        pub(super) listener: SubjectId,
        pub(super) bystander: SubjectId,
        pub(super) stranger: SubjectId,
        pub(super) flood: EntityId,
        pub(super) horn: EntityId,
        pub(super) whisper: AffordanceId,
        pub(super) proclaim: AffordanceId,
        pub(super) recant: AffordanceId,
        pub(super) stair: EdgeId,
    }

    pub(super) const FLOOD_STATEMENT: &str = "The lower hinge is flooding.";
    pub(super) const FLOOD_EVIDENCE: &str = "vault:flood-survey";

    /// A world with one room inside another, four subjects, a fact, and a
    /// channel. Two world-declared speaking entries exercise both audiences: an
    /// addressed co-located `whisper` and a channel-bound `proclaim`.
    pub(super) fn speech_world(kernel: &mut WorldKernel) -> (Speech, WorldSnapshot) {
        let before = kernel.snapshot().unwrap();
        let speak = speak_entry(kernel);
        let person = |handle: &str, label: &str, place: Option<&str>| {
            Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new(handle),
                label: label.into(),
                kind: SubjectKind::Person,
                controller: NewController::NarrativePersona,
                affordances: BTreeSet::from([
                    speak.clone(),
                    Ref::Draft(DraftHandle::new("whisper")),
                    Ref::Draft(DraftHandle::new("proclaim")),
                    Ref::Draft(DraftHandle::new("recant")),
                ]),
                position: place.map(|place| Ref::Draft(DraftHandle::new(place))),
            })
        };
        submit_owner(
            kernel,
            &before,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    evidence: vec![EvidenceRef::new(FLOOD_EVIDENCE)],
                    operations: Vec::new(),
                    declarations: vec![
                        Declaration::Entity(EntityDeclaration {
                            handle: DraftHandle::new("hall"),
                            label: "The Long Hall".into(),
                            kind: EntityKind::Place,
                            container: None,
                        }),
                        Declaration::Entity(EntityDeclaration {
                            handle: DraftHandle::new("yard"),
                            label: "The Cavity Yard".into(),
                            kind: EntityKind::Place,
                            container: Some(Ref::Draft(DraftHandle::new("hall"))),
                        }),
                        Declaration::Route(RouteDeclaration {
                            handle: DraftHandle::new("stair"),
                            label: "The Yard Stair".into(),
                            from: Ref::Draft(DraftHandle::new("yard")),
                            to: Ref::Draft(DraftHandle::new("hall")),
                            access: AccessKind::Public,
                            cost: Cost(1),
                        }),
                        Declaration::Fact(FactDeclaration {
                            handle: DraftHandle::new("flood"),
                            label: "the flooding of the lower hinge".into(),
                            statement: Statement::new(FLOOD_STATEMENT).unwrap(),
                            standing: FactStandingRef::Canonical {
                                evidence: EvidenceRef::new(FLOOD_EVIDENCE),
                            },
                        }),
                        // Addressed speech: the audience is still the room.
                        Declaration::Affordance(AffordanceDeclaration {
                            handle: DraftHandle::new("whisper"),
                            kind: AffordanceKindName("whisper".into()),
                            roles: vec![RoleSpec {
                                role: Role("target".into()),
                                kind: RefKind::Subject(None),
                            }],
                            preconditions: vec![Precondition::CanReach {
                                subject: Role("target".into()),
                                via: AudienceSpec::Colocated,
                            }],
                            effect_slots: Vec::new(),
                            outcome_bands: vec![OutcomeBand {
                                weight: 1,
                                effects: Vec::new(),
                            }],
                            carries_speech: true,
                        }),
                        Declaration::Affordance(AffordanceDeclaration {
                            handle: DraftHandle::new("proclaim"),
                            kind: AffordanceKindName("proclaim".into()),
                            roles: vec![RoleSpec {
                                role: Role("channel".into()),
                                kind: RefKind::Entity(EntityKind::Channel),
                            }],
                            preconditions: vec![Precondition::CanBroadcast {
                                via: AudienceSpec::Channel(Role("channel".into())),
                            }],
                            effect_slots: Vec::new(),
                            outcome_bands: vec![OutcomeBand {
                                weight: 1,
                                effects: Vec::new(),
                            }],
                            carries_speech: true,
                        }),
                        // `Knows` plus the one knowledge operation an affordance
                        // may propose: a subject may only unremember what it
                        // holds with certainty.
                        Declaration::Affordance(AffordanceDeclaration {
                            handle: DraftHandle::new("recant"),
                            kind: AffordanceKindName("recant".into()),
                            roles: vec![RoleSpec {
                                role: Role("fact".into()),
                                kind: RefKind::Entity(EntityKind::Fact),
                            }],
                            preconditions: vec![Precondition::Knows {
                                fact: Role("fact".into()),
                                at_least: Confidence::Certain,
                            }],
                            effect_slots: vec![EffectSlot {
                                op_kind: ComponentOpKind::Forget,
                                roles: vec![Role("actor".into()), Role("fact".into())],
                                bounds: Bounds::None,
                            }],
                            outcome_bands: vec![OutcomeBand {
                                weight: 1,
                                effects: vec![0],
                            }],
                            carries_speech: false,
                        }),
                        person("speaker", "The Hall Speaker", Some("hall")),
                        person("listener", "The Hall Listener", Some("hall")),
                        person("bystander", "The Yard Bystander", Some("yard")),
                        person("stranger", "The Placeless Stranger", None),
                        Declaration::Channel(ChannelDeclaration {
                            handle: DraftHandle::new("horn"),
                            label: "The Temple Horn".into(),
                            reach: ReachRef::Place(Ref::Draft(DraftHandle::new("hall"))),
                            controller: Some(Ref::Draft(DraftHandle::new("speaker"))),
                        }),
                    ],
                },
            },
        );
        let active = activate(kernel);
        let place = |label: &str| {
            active
                .places
                .iter()
                .find(|place| place.label == label)
                .expect("the declared place")
                .id
        };
        let who = |label: &str| {
            active
                .subjects
                .iter()
                .find(|subject| subject.label == label)
                .expect("the declared subject")
                .id
        };
        let fact = *kernel
            .state
            .facts
            .iter()
            .find(|(_, record)| record.statement.as_str() == FLOOD_STATEMENT)
            .expect("the declared fact")
            .0;
        let horn = *kernel
            .state
            .channels
            .keys()
            .next()
            .expect("the declared channel");
        (
            Speech {
                hall: place("The Long Hall"),
                yard: place("The Cavity Yard"),
                speaker: who("The Hall Speaker"),
                listener: who("The Hall Listener"),
                bystander: who("The Yard Bystander"),
                stranger: who("The Placeless Stranger"),
                flood: fact,
                horn,
                whisper: affordance_named(&active, "whisper"),
                proclaim: affordance_named(&active, "proclaim"),
                recant: affordance_named(&active, "recant"),
                stair: active.routes[0].id,
            },
            active,
        )
    }

    /// Submits as owner and returns the complete mismatch set.
    pub(super) fn reject_owner(
        kernel: &mut WorldKernel,
        snapshot: &WorldSnapshot,
        body: CommandBody,
    ) -> Vec<Mismatch> {
        let error = kernel
            .submit(
                command(
                    snapshot,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    body,
                ),
                &auth_principal(owner()),
            )
            .unwrap_err();
        let KernelError::PatchRejected(mismatches) = error else {
            panic!("expected a rejected patch, got {error:?}");
        };
        mismatches
    }

    pub(super) fn operations(operations: Vec<ComponentOp>) -> CommandBody {
        CommandBody::AdmitPatch {
            answers: None,
            patch: WorldPatch {
                declarations: Vec::new(),
                operations,
                evidence: Vec::new(),
            },
        }
    }

    pub(super) fn opportunity_for(
        snapshot: &WorldSnapshot,
        subject_id: SubjectId,
    ) -> DecisionOpportunity {
        snapshot
            .opportunities
            .iter()
            .find(|value| value.scope.subject_id == subject_id)
            .expect("the subject has an opportunity")
            .clone()
    }

    fn opportunity(snapshot: &WorldSnapshot, mode: ControllerMode) -> DecisionOpportunity {
        snapshot
            .opportunities
            .iter()
            .find(|value| value.controller_mode == mode)
            .unwrap()
            .clone()
    }

    fn speak(opportunity: &DecisionOpportunity, text: &str) -> DecisionInvocation {
        DecisionInvocation {
            affordance: opportunity.affordance_ids[0],
            bindings: Vec::new(),
            proposed: Vec::new(),
            speech: Some(Statement::new(text).unwrap()),
        }
    }

    #[test]
    fn external_opportunities_reject_nested_authority_claims() {
        let opportunity = DecisionOpportunity {
            world_id: WorldId::issue(),
            revision: 4,
            scope_digest: ScopeDigest::fixture("sha256:fixture"),
            scope: DecisionScope {
                subject_id: SubjectId::issue(),
            },
            controller_id: ControllerId::issue(),
            controller_mode: ControllerMode::NarrativePersona,
            affordance_ids: vec![AffordanceId::issue()],
        };
        let mut nested = serde_json::to_value(&opportunity).unwrap();
        nested["scope"]["caller"] = serde_json::json!("legacy-owner");
        assert!(serde_json::from_value::<DecisionOpportunity>(nested).is_err());

        let mut outer = serde_json::to_value(&opportunity).unwrap();
        outer["callerId"] = serde_json::json!("legacy-controller");
        assert!(serde_json::from_value::<DecisionOpportunity>(outer).is_err());
    }

    /// A commit elsewhere in the world does not discard a bound proposal: the
    /// binding is the scope digest, and this scope did not change.
    #[test]
    fn an_unchanged_scope_commits_at_a_later_revision() {
        let dir = tempfile::tempdir().unwrap();
        let (mut kernel, _) = WorldKernel::create(
            dir.path().join("world.cc"),
            creation(CommandId::new(), "Unchanged"),
            &auth_principal(owner()),
        )
        .unwrap();
        let topology = admit_topology(&mut kernel);
        let active = activate(&mut kernel);
        let bound = opportunity_for(&active, topology.walker);

        let persona = opportunity(&active, ControllerMode::NarrativePersona);
        let persona_caller = CallerId::Controller(persona.controller_id);
        kernel
            .submit(
                command(
                    &active,
                    CommandId::new(),
                    persona_caller.clone(),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&persona, "The yard is quiet."),
                        opportunity: persona,
                    },
                ),
                &AuthenticatedCaller::fixture(persona_caller),
            )
            .unwrap();
        let later = kernel.snapshot().unwrap();
        assert_eq!(later.revision, active.revision + 1);

        let walker_caller = CallerId::Controller(bound.controller_id);
        let receipt = kernel
            .submit(
                command(
                    &later,
                    CommandId::new(),
                    walker_caller.clone(),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&bound, "I take the ramp."),
                        opportunity: bound.clone(),
                    },
                ),
                &AuthenticatedCaller::fixture(walker_caller),
            )
            .unwrap();
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
        let after = kernel.state.events.clone();
        assert_eq!(after.len(), 2);
        assert_eq!(after[1].revision, bound.revision + 2);
    }

    /// Closing a route incident to the subject's place changes exactly what the
    /// proposal's verification reads, so the bound proposal dies at the digest.
    #[test]
    fn a_changed_scope_is_rejected_with_scope_changed() {
        let dir = tempfile::tempdir().unwrap();
        let (mut kernel, _) = WorldKernel::create(
            dir.path().join("world.cc"),
            creation(CommandId::new(), "Changed"),
            &auth_principal(owner()),
        )
        .unwrap();
        let topology = admit_topology(&mut kernel);
        let active = activate(&mut kernel);
        let bound = opportunity_for(&active, topology.walker);

        submit_owner(
            &mut kernel,
            &active,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: Vec::new(),
                    operations: vec![ComponentOp::CloseRoute {
                        route: Ref::Existing(topology.ramp),
                    }],
                    evidence: Vec::new(),
                },
            },
        );
        let closed = kernel.snapshot().unwrap();
        let walker_caller = CallerId::Controller(bound.controller_id);
        let error = kernel
            .submit(
                command(
                    &closed,
                    CommandId::new(),
                    walker_caller.clone(),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&bound, "I take the ramp."),
                        opportunity: bound.clone(),
                    },
                ),
                &AuthenticatedCaller::fixture(walker_caller),
            )
            .unwrap_err();
        let KernelError::ScopeChanged {
            scope,
            expected,
            actual,
        } = error
        else {
            panic!("expected a changed scope");
        };
        assert_eq!(scope, bound.scope);
        assert_eq!(expected, bound.scope_digest);
        assert_ne!(expected, actual);
        assert_eq!(kernel.snapshot().unwrap(), closed);
    }

    /// The digest covers the scope's controller, its grants, its Position, and
    /// its incident routes, and nothing else in the world.
    #[test]
    fn scope_digest_reads_exactly_its_components() {
        let dir = tempfile::tempdir().unwrap();
        let (mut kernel, _) = WorldKernel::create(
            dir.path().join("world.cc"),
            creation(CommandId::new(), "Components"),
            &auth_principal(owner()),
        )
        .unwrap();
        let topology = admit_topology(&mut kernel);
        activate(&mut kernel);
        let scope = DecisionScope {
            subject_id: topology.walker,
        };
        let base = scope_digest(&kernel.state, scope).unwrap();

        let mut changed_controller = kernel.state.clone();
        let assignment = changed_controller
            .controller_assignments
            .get_mut(&scope)
            .unwrap();
        *assignment = ControllerAssignment::NarrativePersona {
            controller_id: assignment.id(),
        };
        assert_ne!(scope_digest(&changed_controller, scope).unwrap(), base);

        let mut changed_grants = kernel.state.clone();
        let extra = AffordanceId::issue();
        changed_grants
            .affordance_catalog
            .insert(extra, patch::kernel_speak_entry());
        changed_grants
            .affordance_grants
            .entry(scope)
            .or_default()
            .insert(extra);
        assert_ne!(scope_digest(&changed_grants, scope).unwrap(), base);

        // The digest reads the granted entries' definitions, not only their
        // ids: admission reads preconditions and ceilings, so altering an entry
        // must move the digest.
        let mut changed_entry = kernel.state.clone();
        let granted = *kernel.state.affordance_grants[&scope].first().unwrap();
        changed_entry
            .affordance_catalog
            .get_mut(&granted)
            .unwrap()
            .carries_speech = false;
        assert_ne!(scope_digest(&changed_entry, scope).unwrap(), base);

        let mut changed_position = kernel.state.clone();
        changed_position.positions.insert(
            topology.walker,
            Position {
                place: topology.road,
            },
        );
        assert_ne!(scope_digest(&changed_position, scope).unwrap(), base);

        let mut changed_route = kernel.state.clone();
        changed_route
            .edges
            .get_mut(&topology.ramp)
            .unwrap()
            .set_cost(Cost(99));
        assert_ne!(scope_digest(&changed_route, scope).unwrap(), base);

        // The span joins road and gate, so it is incident to neither the walker's
        // place nor its scope; an unrelated subject's decision is invisible too.
        let mut unrelated_route = kernel.state.clone();
        unrelated_route
            .edges
            .get_mut(&topology.span)
            .unwrap()
            .set_cost(Cost(99));
        assert_eq!(scope_digest(&unrelated_route, scope).unwrap(), base);

        let active = kernel.snapshot().unwrap();
        let persona = opportunity(&active, ControllerMode::NarrativePersona);
        let persona_caller = CallerId::Controller(persona.controller_id);
        kernel
            .submit(
                command(
                    &active,
                    CommandId::new(),
                    persona_caller.clone(),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&persona, "Unrelated."),
                        opportunity: persona,
                    },
                ),
                &AuthenticatedCaller::fixture(persona_caller),
            )
            .unwrap();
        assert_eq!(scope_digest(&kernel.state, scope).unwrap(), base);
    }

    #[test]
    fn draft_activation_player_and_autonomous_actions_share_one_reducer() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "  Delvehold  "),
            &auth_principal(owner()),
        )
        .unwrap();
        let genesis = kernel.snapshot().unwrap();
        assert_eq!(genesis.title, "Delvehold");
        assert_eq!(genesis.phase, WorldPhase::Draft);
        assert!(genesis.opportunities.is_empty());

        let active = activate(&mut kernel);
        assert_eq!(active.phase, WorldPhase::Active);
        assert_eq!(active.opportunities.len(), 3);

        let player_opportunity = opportunity(&active, ControllerMode::Human);
        kernel
            .submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&player_opportunity, "I open the door."),
                        opportunity: player_opportunity,
                    },
                ),
                &auth_principal(player()),
            )
            .unwrap();
        let after_player = kernel.snapshot().unwrap();
        assert_eq!(kernel.state.events.len(), 1);
        assert_eq!(
            spoken(&kernel.state, &kernel.state.events[0]),
            Some("I open the door.")
        );

        let persona_opportunity = opportunity(&after_player, ControllerMode::NarrativePersona);
        let persona_caller = CallerId::Controller(persona_opportunity.controller_id);
        kernel
            .submit(
                command(
                    &after_player,
                    CommandId::new(),
                    persona_caller.clone(),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&persona_opportunity, "Then mind the hinge."),
                        opportunity: persona_opportunity,
                    },
                ),
                &AuthenticatedCaller::fixture(persona_caller),
            )
            .unwrap();
        let after_persona = kernel.snapshot().unwrap();
        assert_eq!(kernel.state.events.len(), 2);
        assert!(
            after_persona
                .opportunities
                .iter()
                .all(|value| value.revision == after_persona.revision)
        );
    }

    #[test]
    fn lifecycle_and_controller_authority_fail_closed_without_commits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "Held"),
            &auth_principal(owner()),
        )
        .unwrap();
        let genesis = kernel.snapshot().unwrap();
        assert!(matches!(
            kernel.submit(
                command(
                    &genesis,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    CommandBody::ActivateWorld,
                ),
                &auth_principal(owner())
            ),
            Err(KernelError::MissingApprovals(_))
        ));
        assert_eq!(kernel.journal.commit_count(), 1);

        submit_owner(&mut kernel, &genesis, CommandBody::ApproveDraft);
        let owner_approved = kernel.snapshot().unwrap();
        assert!(matches!(
            kernel.submit(
                command(
                    &owner_approved,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ActivateWorld,
                ),
                &auth_principal(player())
            ),
            Err(KernelError::Unauthorized)
        ));
        assert!(matches!(
            kernel.submit(
                command(
                    &owner_approved,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    CommandBody::ApproveDraft,
                ),
                &auth_principal(owner())
            ),
            Err(KernelError::DraftAlreadyApproved)
        ));
        assert_eq!(kernel.journal.commit_count(), 2);

        kernel
            .submit(
                command(
                    &owner_approved,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ApproveDraft,
                ),
                &auth_principal(player()),
            )
            .unwrap();
        let approved = kernel.snapshot().unwrap();
        submit_owner(&mut kernel, &approved, CommandBody::ActivateWorld);
        let active = kernel.snapshot().unwrap();
        let persona = opportunity(&active, ControllerMode::NarrativePersona);
        let commits_before_forgery = kernel.journal.commit_count();
        assert!(matches!(
            kernel.submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&persona, "Puppet"),
                        opportunity: persona,
                    },
                ),
                &auth_principal(player())
            ),
            Err(KernelError::ControllerMismatch)
        ));
        assert_eq!(kernel.journal.commit_count(), commits_before_forgery);
        assert_eq!(kernel.snapshot().unwrap(), active);
    }

    #[test]
    fn a_tampered_or_unknown_opportunity_never_commits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "Exact"),
            &auth_principal(owner()),
        )
        .unwrap();
        let active = activate(&mut kernel);
        let original = opportunity(&active, ControllerMode::Human);
        // An entry only the operational subject is granted: the human's scope
        // does not hold it, which is what denial means now that an affordance id
        // names a catalog entry rather than one subject's copy of one.
        let denied_invocation = DecisionInvocation {
            affordance: affordance_named(&active, "convene"),
            bindings: Vec::new(),
            proposed: Vec::new(),
            speech: Some(Statement::new("No grant").unwrap()),
        };
        assert!(matches!(
            kernel.submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ExerciseDecision {
                        invocation: denied_invocation,
                        opportunity: original.clone(),
                    },
                ),
                &auth_principal(player())
            ),
            Err(KernelError::AffordanceDenied)
        ));
        // A claimed affordance the world did not grant carries no authority: the
        // kernel exercises the opportunity it derives, never the one submitted.
        let forged_affordance = AffordanceId::issue();
        let mut tampered = original.clone();
        tampered.affordance_ids.push(forged_affordance);
        assert!(matches!(
            kernel.submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ExerciseDecision {
                        invocation: DecisionInvocation {
                            affordance: forged_affordance,
                            bindings: Vec::new(),
                            proposed: Vec::new(),
                            speech: Some(Statement::new("Forged").unwrap()),
                        },
                        opportunity: tampered,
                    },
                ),
                &auth_principal(player())
            ),
            Err(KernelError::AffordanceDenied)
        ));
        assert_eq!(kernel.snapshot().unwrap(), active);

        let mut forged_scope = original.clone();
        forged_scope.scope_digest = ScopeDigest::fixture("sha256:not-this-scope");
        assert!(matches!(
            kernel.submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&original, "No"),
                        opportunity: forged_scope,
                    },
                ),
                &auth_principal(player())
            ),
            Err(KernelError::ScopeChanged { .. })
        ));
        assert_eq!(kernel.snapshot().unwrap(), active);

        let unknown_scope = DecisionOpportunity {
            scope: DecisionScope {
                subject_id: SubjectId::issue(),
            },
            ..original.clone()
        };
        assert!(matches!(
            kernel.submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ExerciseDecision {
                        invocation: speak(&original, "No"),
                        opportunity: unknown_scope,
                    },
                ),
                &auth_principal(player())
            ),
            Err(KernelError::OpportunityMismatch)
        ));
        assert_eq!(kernel.snapshot().unwrap(), active);
    }

    #[test]
    fn create_submit_restart_and_replay_are_exact() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let creation = creation(CommandId::new(), "First");
        let (mut kernel, creation_receipt) =
            WorldKernel::create(&path, creation.clone(), &auth_principal(owner())).unwrap();
        let active = activate(&mut kernel);
        let decision = opportunity(&active, ControllerMode::OperationalAgent);
        let caller = CallerId::Controller(decision.controller_id);
        let command = command(
            &active,
            CommandId::new(),
            caller.clone(),
            CommandBody::ExerciseDecision {
                invocation: speak(&decision, "The vote carries."),
                opportunity: decision,
            },
        );
        let applied = kernel
            .submit(
                command.clone(),
                &AuthenticatedCaller::fixture(caller.clone()),
            )
            .unwrap();
        let SubmitReceipt::Applied(receipt) = applied else {
            panic!("expected applied receipt")
        };
        let accepted = kernel.snapshot().unwrap();
        drop(kernel);

        let (mut reopened, retried_creation_receipt) =
            WorldKernel::create(&path, creation, &auth_principal(owner())).unwrap();
        assert_eq!(retried_creation_receipt, creation_receipt);
        assert_eq!(reopened.snapshot().unwrap(), accepted);
        assert_eq!(
            reopened
                .submit(command, &AuthenticatedCaller::fixture(caller))
                .unwrap(),
            SubmitReceipt::AlreadyApplied(receipt)
        );
        drop(reopened);
        let reopened = WorldKernel::open(&path, accepted.world_id).unwrap();
        assert_eq!(reopened.snapshot().unwrap(), accepted);
    }

    #[test]
    fn controller_receipt_is_bound_to_the_exact_opportunity_and_invocation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "Receipt Scope"),
            &auth_principal(owner()),
        )
        .unwrap();
        let active = activate(&mut kernel);
        let decision = opportunity(&active, ControllerMode::OperationalAgent);
        let other = opportunity(&active, ControllerMode::NarrativePersona);
        let command_id = CommandId::new();
        let caller = CallerId::Controller(decision.controller_id);
        let invocation = speak(&decision, "The record is exact.");
        kernel
            .submit(
                command(
                    &active,
                    command_id,
                    caller.clone(),
                    CommandBody::ExerciseDecision {
                        invocation: invocation.clone(),
                        opportunity: decision.clone(),
                    },
                ),
                &AuthenticatedCaller::fixture(caller),
            )
            .unwrap();

        let receipt = kernel
            .controller_receipt(command_id, &decision, &invocation)
            .unwrap()
            .expect("committed controller command has a receipt");
        assert_eq!(receipt.command_id, command_id);
        assert!(
            kernel
                .controller_receipt(CommandId::new(), &decision, &invocation)
                .unwrap()
                .is_none()
        );
        assert!(matches!(
            kernel.controller_receipt(command_id, &other, &invocation),
            Err(KernelError::CommandIdConflict)
        ));
        let altered_invocation = speak(&decision, "A different proposal.");
        assert!(matches!(
            kernel.controller_receipt(command_id, &decision, &altered_invocation),
            Err(KernelError::CommandIdConflict)
        ));
    }

    #[test]
    fn controller_decline_consumes_only_its_exact_opportunity() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "Decline Scope"),
            &auth_principal(owner()),
        )
        .unwrap();
        let active = activate(&mut kernel);
        let decision = opportunity(&active, ControllerMode::OperationalAgent);
        let other = opportunity(&active, ControllerMode::NarrativePersona);
        let caller = CallerId::Controller(decision.controller_id);

        assert!(matches!(
            kernel.submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Controller(other.controller_id),
                    CommandBody::DeclineDecision {
                        opportunity: decision.clone(),
                    },
                ),
                &AuthenticatedCaller::fixture(CallerId::Controller(other.controller_id)),
            ),
            Err(KernelError::ControllerMismatch)
        ));
        assert_eq!(kernel.snapshot().unwrap(), active);

        let command_id = CommandId::new();
        let decline = command(
            &active,
            command_id,
            caller.clone(),
            CommandBody::DeclineDecision {
                opportunity: decision.clone(),
            },
        );
        let SubmitReceipt::Applied(receipt) = kernel
            .submit(
                decline.clone(),
                &AuthenticatedCaller::fixture(caller.clone()),
            )
            .unwrap()
        else {
            panic!("exact decline was not applied")
        };
        let after = kernel.snapshot().unwrap();
        assert_eq!(after.revision, active.revision + 1);
        assert!(kernel.state.events.is_empty());
        let refreshed = opportunity(&after, ControllerMode::OperationalAgent);
        assert_ne!(refreshed, decision);
        assert_eq!(
            kernel
                .controller_decline_receipt(command_id, &decision)
                .unwrap(),
            Some(receipt.clone())
        );
        assert!(matches!(
            kernel.controller_decline_receipt(command_id, &other),
            Err(KernelError::CommandIdConflict)
        ));
        assert!(matches!(
            kernel.controller_receipt(
                command_id,
                &decision,
                &speak(&decision, "A decline is not speech."),
            ),
            Err(KernelError::CommandIdConflict)
        ));
        assert_eq!(
            kernel
                .submit(decline, &AuthenticatedCaller::fixture(caller.clone()))
                .unwrap(),
            SubmitReceipt::AlreadyApplied(receipt.clone())
        );
        assert_eq!(kernel.snapshot().unwrap(), after);
        drop(kernel);
        let reopened = WorldKernel::open(&path, after.world_id).unwrap();
        assert_eq!(reopened.snapshot().unwrap(), after);
        assert_eq!(
            reopened
                .controller_decline_receipt(
                    command_id,
                    &opportunity(&active, ControllerMode::OperationalAgent),
                )
                .unwrap(),
            Some(receipt)
        );
    }

    #[test]
    fn duplicate_and_stale_commands_do_not_commit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "Still"),
            &auth_principal(owner()),
        )
        .unwrap();
        let genesis = kernel.snapshot().unwrap();
        assert!(matches!(
            submit_owner(&mut kernel, &genesis, CommandBody::ApproveDraft),
            SubmitReceipt::Applied(_)
        ));
        let after = kernel.snapshot().unwrap();
        assert!(matches!(
            kernel.submit(
                command(
                    &after,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    CommandBody::ApproveDraft,
                ),
                &auth_principal(owner())
            ),
            Err(KernelError::DraftAlreadyApproved)
        ));
        let stale = command(
            &genesis,
            CommandId::new(),
            CallerId::Principal(owner()),
            CommandBody::ApproveDraft,
        );
        assert!(matches!(
            kernel.submit(stale, &auth_principal(owner())),
            Err(KernelError::RevisionMismatch { .. })
        ));
        assert_eq!(kernel.snapshot().unwrap(), after);
        assert_eq!(kernel.journal.commit_count(), 2);
    }

    #[test]
    fn durable_idempotency_precedes_revision_and_detects_conflicts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "A"),
            &auth_principal(owner()),
        )
        .unwrap();
        let genesis = kernel.snapshot().unwrap();
        let id = CommandId::new();
        let first = command(
            &genesis,
            id,
            CallerId::Principal(owner()),
            CommandBody::ApproveDraft,
        );
        let first_receipt = kernel
            .submit(first.clone(), &auth_principal(owner()))
            .unwrap();
        let second_snapshot = kernel.snapshot().unwrap();
        kernel
            .submit(
                command(
                    &second_snapshot,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ApproveDraft,
                ),
                &auth_principal(player()),
            )
            .unwrap();
        let after_second = kernel.snapshot().unwrap();
        assert_eq!(
            kernel.submit(first, &auth_principal(owner())).unwrap(),
            match first_receipt {
                SubmitReceipt::Applied(value) => SubmitReceipt::AlreadyApplied(value),
                _ => unreachable!(),
            }
        );
        let conflict = command(
            &genesis,
            id,
            CallerId::Principal(owner()),
            CommandBody::ActivateWorld,
        );
        assert!(matches!(
            kernel.submit(conflict, &auth_principal(owner())),
            Err(KernelError::CommandIdConflict)
        ));
        assert_eq!(kernel.snapshot().unwrap(), after_second);
    }

    #[test]
    fn creation_retry_is_bound_to_the_exact_raw_command() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let id = CommandId::new();
        let raw = creation(id, "  Canonical Result  ");
        let (kernel, first) =
            WorldKernel::create(&path, raw.clone(), &auth_principal(owner())).unwrap();
        assert_eq!(kernel.snapshot().unwrap().title, "Canonical Result");
        drop(kernel);

        let (kernel, retry) =
            WorldKernel::create(&path, raw.clone(), &auth_principal(owner())).unwrap();
        assert_eq!(retry, first);
        drop(kernel);

        let mut merely_equivalent = raw;
        merely_equivalent.title = "Canonical Result".into();
        assert!(matches!(
            WorldKernel::create(&path, merely_equivalent, &auth_principal(owner())),
            Err(KernelError::CreationConflict)
        ));
    }

    #[test]
    fn invalid_seed_never_allocates_a_world() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let mut invalid = creation(CommandId::new(), "Nope");
        invalid.patch.declarations[3] = invalid.patch.declarations[2].clone();
        let Err(KernelError::PatchRejected(rejected)) =
            WorldKernel::create(&path, invalid, &auth_principal(owner()))
        else {
            panic!("expected a rejected creation patch");
        };
        assert_eq!(
            rejected,
            vec![Mismatch::DuplicateHandle {
                handle: DraftHandle::new("player")
            }]
        );
        assert!(
            WorldKernel::create(
                &path,
                creation(CommandId::new(), "Valid"),
                &auth_principal(owner())
            )
            .is_ok()
        );
    }

    #[test]
    fn a_second_live_owner_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let creation = creation(CommandId::new(), "Held");
        let (kernel, _) =
            WorldKernel::create(&path, creation.clone(), &auth_principal(owner())).unwrap();
        let world_id = kernel.snapshot().unwrap().world_id;
        assert!(matches!(
            WorldKernel::open(&path, world_id),
            Err(KernelError::Store(_))
        ));
        assert!(matches!(
            WorldKernel::create(&path, creation.clone(), &auth_principal(owner())),
            Err(KernelError::Store(_))
        ));
        drop(kernel);
        let kernel = WorldKernel::open(&path, world_id).unwrap();
        drop(kernel);
        assert!(matches!(
            WorldKernel::open(&path, WorldId::issue()),
            Err(KernelError::OpenedWorldMismatch)
        ));
    }

    #[test]
    fn lost_post_commit_ack_poisons_until_reopen_and_exact_retry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "Certain"),
            &auth_principal(owner()),
        )
        .unwrap();
        let genesis = kernel.snapshot().unwrap();
        let uncertain = CommandId::new();
        let attempted = command(
            &genesis,
            uncertain,
            CallerId::Principal(owner()),
            CommandBody::ApproveDraft,
        );
        kernel.journal.fail_after_durable_commit_for_test();
        assert!(matches!(
            kernel.submit(attempted.clone(), &auth_principal(owner())),
            Err(KernelError::RecoveryRequired { command_id }) if command_id == uncertain
        ));
        assert!(matches!(
            kernel.snapshot(),
            Err(KernelError::RecoveryRequired { command_id }) if command_id == uncertain
        ));
        drop(kernel);

        let mut reopened = WorldKernel::open(&path, genesis.world_id).unwrap();
        let durable = reopened.snapshot().unwrap();
        assert_eq!(durable.revision, 1);
        assert_eq!(durable.draft_approvals, BTreeSet::from([owner()]));
        assert!(matches!(
            reopened
                .submit(attempted, &auth_principal(owner()))
                .unwrap(),
            SubmitReceipt::AlreadyApplied(_)
        ));
    }

    #[test]
    fn lost_genesis_ack_reopens_as_the_same_world_and_exact_receipt() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let command_id = CommandId::new();
        let input = creation(command_id, "Durable Genesis");
        let authenticated = auth_principal(owner());
        let mut empty = match journal::WorldJournal::open_owner(&path).unwrap() {
            journal::JournalOpen::Empty(empty) => empty,
            journal::JournalOpen::Live { .. } => panic!("fixture store must be empty"),
        };
        empty.fail_after_durable_initialize_for_test();
        let prepared = prepare_creation(input.clone(), &authenticated).unwrap();
        assert!(matches!(
            WorldKernel::initialize(empty, prepared),
            Err(KernelError::RecoveryRequired { command_id: uncertain })
                if uncertain == command_id
        ));

        let (reopened, receipt) = WorldKernel::create(&path, input, &authenticated).unwrap();
        let snapshot = reopened.snapshot().unwrap();
        assert_eq!(receipt.command_id, command_id);
        assert_eq!(receipt.world_id, snapshot.world_id);
        assert_eq!(snapshot.revision, 0);
        assert_eq!(snapshot.title, "Durable Genesis");
    }

    #[test]
    fn replacing_the_store_path_revokes_the_live_owner() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let displaced = dir.path().join("displaced.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "Pinned"),
            &auth_principal(owner()),
        )
        .unwrap();
        let snapshot = kernel.snapshot().unwrap();
        std::fs::rename(&path, &displaced).unwrap();
        std::fs::File::create(&path).unwrap();
        assert!(matches!(kernel.snapshot(), Err(KernelError::OwnershipLost)));
        assert!(matches!(
            kernel.submit(
                command(
                    &snapshot,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    CommandBody::ApproveDraft,
                ),
                &auth_principal(owner())
            ),
            Err(KernelError::OwnershipLost)
        ));
        std::fs::remove_file(&path).unwrap();
        std::fs::rename(&displaced, &path).unwrap();
        assert!(matches!(kernel.snapshot(), Err(KernelError::OwnershipLost)));
        drop(kernel);
        let reopened = WorldKernel::open(&path, snapshot.world_id).unwrap();
        assert_eq!(reopened.snapshot().unwrap(), snapshot);
    }

    #[test]
    fn sealed_authentication_cannot_be_replaced_by_a_command_claim() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.cc");
        let (mut kernel, _) = WorldKernel::create(
            &path,
            creation(CommandId::new(), "Owned"),
            &auth_principal(owner()),
        )
        .unwrap();
        let snapshot = kernel.snapshot().unwrap();
        let forged = command(
            &snapshot,
            CommandId::new(),
            CallerId::Principal(player()),
            CommandBody::ApproveDraft,
        );
        assert!(matches!(
            kernel.submit(forged, &auth_principal(owner())),
            Err(KernelError::AuthenticationMismatch)
        ));
        assert_eq!(kernel.snapshot().unwrap(), snapshot);
    }

    /// Soul falsification: the preimage must exclude the revision, the phase,
    /// and every component outside the scope. Each step below is a real commit
    /// that moves the world, not a hand-edited state.
    #[test]
    fn soul_scope_digest_excludes_revision_phase_and_everything_outside_the_scope() {
        let dir = tempfile::tempdir().unwrap();
        let (mut kernel, _) = WorldKernel::create(
            dir.path().join("world.cc"),
            creation(CommandId::new(), "Preimage"),
            &auth_principal(owner()),
        )
        .unwrap();
        let topology = admit_topology(&mut kernel);
        let scope = DecisionScope {
            subject_id: topology.walker,
        };
        let base = scope_digest(&kernel.state, scope).unwrap();
        let start_revision = kernel.state.revision;

        // A new place elsewhere, a route that touches neither the walker's
        // place nor it, and a second placed subject: all outside the scope.
        let speak = speak_entry(&kernel);
        let before = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &before,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: vec![
                        Declaration::Entity(EntityDeclaration {
                            handle: DraftHandle::new("annex"),
                            label: "The Annex".into(),
                            kind: EntityKind::Place,
                            container: None,
                        }),
                        Declaration::Route(RouteDeclaration {
                            handle: DraftHandle::new("annexway"),
                            label: "The Annex Way".into(),
                            from: Ref::Existing(topology.road),
                            to: Ref::Draft(DraftHandle::new("annex")),
                            access: AccessKind::Public,
                            cost: Cost(3),
                        }),
                        Declaration::Subject(SubjectDeclaration {
                            handle: DraftHandle::new("runner"),
                            label: "The Runner".into(),
                            kind: SubjectKind::Person,
                            controller: NewController::OperationalAgent,
                            affordances: BTreeSet::from([speak.clone()]),
                            position: Some(Ref::Existing(topology.road)),
                        }),
                    ],
                    operations: Vec::new(),
                    evidence: Vec::new(),
                },
            },
        );
        assert_ne!(kernel.state.revision, start_revision);
        assert_eq!(scope_digest(&kernel.state, scope).unwrap(), base);

        // Activation changes the phase, the approvals, and the revision.
        let active = activate(&mut kernel);
        assert_eq!(active.phase, WorldPhase::Active);
        assert_eq!(scope_digest(&kernel.state, scope).unwrap(), base);

        // Another subject's Position moving is not this scope's business.
        let annexway = *kernel
            .state
            .edges
            .iter()
            .find(|(_, record)| record.label() == "The Annex Way")
            .expect("the declared route")
            .0;
        let runner = *kernel
            .state
            .subjects
            .iter()
            .find(|(_, subject)| subject.label == "The Runner")
            .expect("the declared subject")
            .0;
        let active = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &active,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: Vec::new(),
                    operations: vec![ComponentOp::Relocate {
                        subject: Ref::Existing(runner),
                        via: Ref::Existing(annexway),
                    }],
                    evidence: Vec::new(),
                },
            },
        );
        assert_eq!(
            kernel.state.positions.get(&runner),
            Some(&Position {
                place: kernel
                    .state
                    .entities
                    .iter()
                    .find(|(_, record)| record.label == "The Annex")
                    .map(|(id, _)| *id)
                    .unwrap()
            })
        );
        assert_eq!(scope_digest(&kernel.state, scope).unwrap(), base);

        // A route incident to the walker's own place is inside the scope.
        let moved = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &moved,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: Vec::new(),
                    operations: vec![ComponentOp::CloseRoute {
                        route: Ref::Existing(topology.ramp),
                    }],
                    evidence: Vec::new(),
                },
            },
        );
        assert_ne!(scope_digest(&kernel.state, scope).unwrap(), base);
    }

    /// Soul falsification: a claimed opportunity whose `affordance_ids` name
    /// grants this scope does not hold cannot widen authority. The kernel reads
    /// the derived list, so the forged one is ignored and the invocation is
    /// denied on the real grants.
    #[test]
    fn soul_a_forged_affordance_list_cannot_widen_authority() {
        let dir = tempfile::tempdir().unwrap();
        let (mut kernel, _) = WorldKernel::create(
            dir.path().join("world.cc"),
            creation(CommandId::new(), "Affordances"),
            &auth_principal(owner()),
        )
        .unwrap();
        let active = activate(&mut kernel);
        let persona = opportunity(&active, ControllerMode::NarrativePersona);
        let operator = opportunity(&active, ControllerMode::OperationalAgent);

        // The persona claims the operator's entry and lists it as its own.
        let convened = affordance_named(&active, "convene");
        assert!(operator.affordance_ids.contains(&convened));
        assert!(!persona.affordance_ids.contains(&convened));
        let mut forged = persona.clone();
        forged.affordance_ids = vec![convened, persona.affordance_ids[0]];
        let caller = CallerId::Controller(persona.controller_id);
        let error = kernel
            .submit(
                command(
                    &active,
                    CommandId::new(),
                    caller.clone(),
                    CommandBody::ExerciseDecision {
                        opportunity: forged,
                        invocation: DecisionInvocation {
                            affordance: convened,
                            bindings: Vec::new(),
                            proposed: Vec::new(),
                            speech: Some(Statement::new("Not my grant.").unwrap()),
                        },
                    },
                ),
                &AuthenticatedCaller::fixture(caller),
            )
            .unwrap_err();
        assert!(matches!(error, KernelError::AffordanceDenied));
        assert_eq!(kernel.snapshot().unwrap(), active);
    }

    /// Soul: an `AffordanceId` now names a catalog entry, not a subject's copy
    /// of one — the three genesis subjects share Speak's single id. So the id
    /// cannot be what gates: grant membership for the invoking scope is. The
    /// same `convene` id commits for the scope that holds it and is denied for
    /// one that does not.
    #[test]
    fn soul_a_shared_catalog_id_is_gated_by_the_scope_grant() {
        let dir = tempfile::tempdir().unwrap();
        let (mut kernel, _) = WorldKernel::create(
            dir.path().join("world.cc"),
            creation(CommandId::new(), "SharedCatalogIds"),
            &auth_principal(owner()),
        )
        .unwrap();
        let active = activate(&mut kernel);
        let speak = affordance_named(&active, "speak");
        let convened = affordance_named(&active, "convene");

        // One entry, one id, every genesis subject a grantee.
        assert_eq!(active.subjects.len(), 3);
        assert!(
            active
                .subjects
                .iter()
                .all(|subject| subject.affordances.contains(&speak))
        );
        assert!(
            active
                .opportunities
                .iter()
                .all(|opportunity| opportunity.affordance_ids.contains(&speak))
        );
        let holders: Vec<SubjectId> = active
            .subjects
            .iter()
            .filter(|subject| subject.affordances.contains(&convened))
            .map(|subject| subject.id)
            .collect();
        assert_eq!(holders.len(), 1);

        let invoke = |kernel: &mut WorldKernel, snapshot: &WorldSnapshot, subject: SubjectId| {
            let opportunity = opportunity_for(snapshot, subject);
            let caller = CallerId::Controller(opportunity.controller_id);
            kernel.submit(
                command(
                    snapshot,
                    CommandId::new(),
                    caller.clone(),
                    CommandBody::ExerciseDecision {
                        opportunity,
                        invocation: DecisionInvocation {
                            affordance: convened,
                            bindings: Vec::new(),
                            proposed: Vec::new(),
                            speech: Some(Statement::new("The council convenes.").unwrap()),
                        },
                    },
                ),
                &AuthenticatedCaller::fixture(caller),
            )
        };

        // The grantee's scope commits with that id.
        assert!(matches!(
            invoke(&mut kernel, &active, holders[0]).unwrap(),
            SubmitReceipt::Applied(_)
        ));

        // A scope that does not hold the entry is denied the very same id, and
        // nothing commits.
        let after = kernel.snapshot().unwrap();
        // A controller-driven scope, so the denial is the grant check rather
        // than the human lane's caller check reached first.
        let stranger = after
            .subjects
            .iter()
            .find(|subject| {
                !subject.affordances.contains(&convened) && subject.human_controller.is_none()
            })
            .expect("a controller-driven subject without the grant")
            .id;
        assert!(matches!(
            invoke(&mut kernel, &after, stranger).unwrap_err(),
            KernelError::AffordanceDenied
        ));
        assert_eq!(kernel.snapshot().unwrap(), after);
    }

    #[test]
    fn a_stale_grant_rejects_but_a_distant_route_reaches_admission_instead() {
        let dir = tempfile::tempdir().unwrap();
        let (mut kernel, _) = WorldKernel::create(
            dir.path().join("world.cc"),
            creation(CommandId::new(), "BindingAndAdmission"),
            &auth_principal(owner()),
        )
        .unwrap();
        let topology = admit_topology(&mut kernel);
        let active = activate(&mut kernel);
        let bound = opportunity_for(&active, topology.walker);
        let speak = affordance_named(&active, "speak");

        // Binding is scope-digest equality, and the digest reads the granted
        // entries' definitions: altering one moves the digest, so a proposal
        // bound before that change no longer binds.
        let mut altered = kernel.state.clone();
        altered
            .affordance_catalog
            .get_mut(&speak)
            .expect("the Speak entry")
            .carries_speech = false;
        assert_ne!(
            scope_digest(&altered, bound.scope).unwrap(),
            scope_digest(&kernel.state, bound.scope).unwrap()
        );

        // A route the actor does not stand beside is outside the digest, so
        // closing it leaves the binding intact. That is deliberate: admission
        // re-reads the live graph at commit, so nothing commits on stale
        // topology and a proposal is not invalidated by distant traffic.
        let before_digest = scope_digest(&kernel.state, bound.scope).unwrap();
        let before = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &before,
            operations(vec![ComponentOp::CloseRoute {
                route: Ref::Existing(topology.span),
            }]),
        );
        assert_eq!(
            scope_digest(&kernel.state, bound.scope).unwrap(),
            before_digest,
            "a route outside the actor's incident set does not rebind the proposal"
        );

        let snapshot = kernel.snapshot().unwrap();
        let receipt = kernel
            .submit(
                command(
                    &snapshot,
                    CommandId::new(),
                    CallerId::Controller(bound.controller_id),
                    CommandBody::ExerciseDecision {
                        opportunity: bound.clone(),
                        invocation: DecisionInvocation {
                            affordance: speak,
                            bindings: Vec::new(),
                            proposed: Vec::new(),
                            speech: Some(Statement::new("Still here.").unwrap()),
                        },
                    },
                ),
                &AuthenticatedCaller::fixture(CallerId::Controller(bound.controller_id)),
            )
            .expect("a distant route closing does not rebind this proposal");
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
    }

    /// The mvp doc's disjointness requirement: an institution with an
    /// operational organ and a person-shaped voice is two subjects joined by an
    /// office, not one subject with two controllers.
    #[test]
    fn an_institution_and_its_voice_are_two_subjects_joined_by_an_office() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = crate::world::custody_tests::custody_kernel(directory.path(), "Two");
        let (_, civic, active) = civic_world(&mut kernel);

        let treasury = opportunity_for(&active, civic.treasury);
        let reeve = opportunity_for(&active, civic.reeve);
        assert_ne!(treasury.controller_id, reeve.controller_id);
        assert_eq!(
            active
                .opportunities
                .iter()
                .filter(|value| value.scope.subject_id == civic.treasury)
                .count(),
            1
        );
        assert_eq!(
            active
                .opportunities
                .iter()
                .filter(|value| value.scope.subject_id == civic.reeve)
                .count(),
            1
        );

        // Both are authorized over the hall — one directly, one by delegation —
        // and neither appears in the other's components.
        let over_hall = AuthorityGrant {
            kind: authority_kind(LEVY_KIND),
            over: AuthorityTarget::PlaceSubtree(civic.hall),
        };
        assert!(subject_authority(&kernel.state, civic.treasury).contains(&over_hall));
        assert!(subject_authority(&kernel.state, civic.reeve).contains(&over_hall));
        assert!(kernel.state.authority.get(&civic.reeve).is_none());
        assert!(
            scope_components(&kernel.state, civic.treasury)
                .delegated
                .is_empty()
        );
        assert!(
            scope_components(&kernel.state, civic.reeve)
                .authority
                .is_empty()
        );
    }

    /// A levy from catalog to conservation: one lowered `Transfer`, the ledger
    /// total unchanged, and no other partition touched.
    #[test]
    fn a_levy_round_trips_from_catalog_to_conservation() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = crate::world::custody_tests::custody_kernel(directory.path(), "Levy");
        let (_, civic, active) = civic_world(&mut kernel);
        let before_total: u64 = kernel
            .state
            .holdings
            .values()
            .filter_map(|held| held.get(&civic.grain))
            .map(|quantity| quantity.0)
            .sum();
        let positions = kernel.state.positions.clone();
        let edges = kernel.state.edges.clone();
        let entities = kernel.state.entities.clone();

        let opportunity = opportunity_for(&active, civic.treasury);
        let caller = CallerId::Controller(opportunity.controller_id);
        let receipt = kernel
            .submit(
                command(
                    &active,
                    CommandId::new(),
                    caller.clone(),
                    CommandBody::ExerciseDecision {
                        opportunity,
                        invocation: DecisionInvocation {
                            affordance: affordance_named(&active, "levy"),
                            bindings: vec![
                                RoleBinding {
                                    role: Role("payer".into()),
                                    target: Target::Subject(civic.farmer),
                                },
                                RoleBinding {
                                    role: Role("resource".into()),
                                    target: Target::Entity(civic.grain),
                                },
                            ],
                            proposed: vec![ProposedEffect {
                                slot: 0,
                                magnitude: Magnitude::Quantity(Quantity(3)),
                            }],
                            speech: None,
                        },
                    },
                ),
                &AuthenticatedCaller::fixture(caller),
            )
            .unwrap();
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));

        let held = |holder: SubjectId| {
            kernel
                .state
                .holdings
                .get(&holder)
                .and_then(|held| held.get(&civic.grain))
                .map_or(0, |quantity| quantity.0)
        };
        assert_eq!(held(civic.farmer), OPENING_BALANCE - 3);
        assert_eq!(held(civic.treasury), 3);
        let after_total: u64 = kernel
            .state
            .holdings
            .values()
            .filter_map(|held| held.get(&civic.grain))
            .map(|quantity| quantity.0)
            .sum();
        assert_eq!(before_total, after_total);
        assert_eq!(
            kernel.state.events.last().unwrap().effects,
            vec![ResolvedOp::Transfer {
                from: civic.farmer,
                to: civic.treasury,
                resource: civic.grain,
                qty: Quantity(3),
            }]
        );
        assert_eq!(kernel.state.positions, positions);
        assert_eq!(kernel.state.edges, edges);
        assert_eq!(kernel.state.entities, entities);
    }

    /// Representation is person-and-institution shaped, and that is the check
    /// that stops the two-subject shape from collapsing back into one.
    #[test]
    fn office_admission_is_person_and_institution_shaped() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = crate::world::custody_tests::custody_kernel(directory.path(), "Offices");
        let (_, civic, _) = civic_world(&mut kernel);
        let refuse = |kernel: &mut WorldKernel, ops: Vec<ComponentOp>| {
            let before = kernel.snapshot().unwrap();
            reject_owner(kernel, &before, operations(ops))
        };

        // An institution cannot occupy its own office.
        assert_eq!(
            refuse(
                &mut kernel,
                vec![ComponentOp::InstallIncumbent {
                    institution: Ref::Existing(civic.treasury),
                    office: office(BAILIFF_OFFICE),
                    incumbent: Ref::Existing(civic.treasury),
                }]
            ),
            vec![Mismatch::OfficeHolderNotPerson { operation: 0 }]
        );
        // An office cannot be opened on a person.
        assert_eq!(
            refuse(
                &mut kernel,
                vec![ComponentOp::OpenOffice {
                    institution: Ref::Existing(civic.reeve),
                    office: office(WARDEN_OFFICE),
                    delegated: BTreeSet::from([authority_kind(LEVY_KIND)]),
                }]
            ),
            vec![Mismatch::OfficeOnNonInstitution { operation: 0 }]
        );
        // One person, at most one office per institution.
        assert_eq!(
            refuse(
                &mut kernel,
                vec![ComponentOp::InstallIncumbent {
                    institution: Ref::Existing(civic.treasury),
                    office: office(BAILIFF_OFFICE),
                    incumbent: Ref::Existing(civic.reeve),
                }]
            ),
            vec![Mismatch::DuplicateIncumbency { operation: 0 }]
        );
    }

    /// Disjointness in both directions: overlap inside one subject's effective
    /// authority is refused, overlap between subjects is layered government.
    #[test]
    fn overlapping_jurisdiction_within_one_subject_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = crate::world::custody_tests::custody_kernel(directory.path(), "Overlap");
        let (_, civic, _) = civic_world(&mut kernel);
        let refuse = |kernel: &mut WorldKernel, ops: Vec<ComponentOp>| {
            let before = kernel.snapshot().unwrap();
            reject_owner(kernel, &before, operations(ops))
        };
        let admit = |kernel: &mut WorldKernel, ops: Vec<ComponentOp>| {
            let before = kernel.snapshot().unwrap();
            submit_owner(kernel, &before, operations(ops))
        };

        // A direct grant under what the reeve's office already lends.
        assert_eq!(
            refuse(
                &mut kernel,
                vec![grant_to(civic.reeve, LEVY_KIND, over_place(civic.chamber))]
            ),
            vec![Mismatch::OverlappingJurisdiction { operation: 0 }]
        );
        // Two direct grants of one kind, nested, in one patch.
        assert_eq!(
            refuse(
                &mut kernel,
                vec![
                    grant_to(civic.farmer, LEVY_KIND, over_place(civic.hall)),
                    grant_to(civic.farmer, LEVY_KIND, over_place(civic.chamber)),
                ]
            ),
            vec![Mismatch::OverlappingJurisdiction { operation: 1 }]
        );
        // An incumbency that would lend the farmer ground it already holds.
        admit(
            &mut kernel,
            vec![grant_to(civic.farmer, LEVY_KIND, over_place(civic.chamber))],
        );
        assert_eq!(
            refuse(
                &mut kernel,
                vec![ComponentOp::InstallIncumbent {
                    institution: Ref::Existing(civic.treasury),
                    office: office(WARDEN_OFFICE),
                    incumbent: Ref::Existing(civic.farmer),
                }]
            ),
            vec![Mismatch::OverlappingJurisdiction { operation: 0 }]
        );

        // A different kind over the same ground commits, and so does the same
        // kind held by a different subject: two holders authorized over one
        // target is not a contradiction, because nothing arbitrates.
        assert!(matches!(
            admit(
                &mut kernel,
                vec![
                    grant_to(civic.reeve, JUDGE_KIND, over_place(civic.chamber)),
                    grant_to(civic.outsider, LEVY_KIND, over_place(civic.chamber)),
                ]
            ),
            SubmitReceipt::Applied(_)
        ));
    }

    /// Contract 6 on the civic surface: one patch, the complete sorted set, no
    /// allocation, and repairing exactly those failures commits.
    #[test]
    fn civic_operations_are_rejected_with_the_complete_set() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = crate::world::custody_tests::custody_kernel(directory.path(), "CivicSet");
        let (_, civic, _) = civic_world(&mut kernel);
        let before = kernel.snapshot().unwrap();
        let broken = vec![
            ComponentOp::OpenOffice {
                institution: Ref::Existing(civic.treasury),
                office: office("steward"),
                delegated: BTreeSet::new(),
            },
            ComponentOp::InstallIncumbent {
                institution: Ref::Existing(civic.treasury),
                office: office("provost"),
                incumbent: Ref::Existing(civic.farmer),
            },
            ComponentOp::CloseForum {
                grievance: grievance("eviction"),
            },
            ComponentOp::OpenOffice {
                institution: Ref::Existing(civic.treasury),
                office: office("Chief Steward"),
                delegated: BTreeSet::from([authority_kind(JUDGE_KIND)]),
            },
            ComponentOp::InstallIncumbent {
                institution: Ref::Existing(civic.treasury),
                office: office(BAILIFF_OFFICE),
                incumbent: Ref::Existing(civic.treasury),
            },
            grant_to(civic.reeve, LEVY_KIND, over_place(civic.chamber)),
        ];
        let mut expected = vec![
            Mismatch::EmptyDelegation { operation: 0 },
            Mismatch::UnknownOffice { operation: 1 },
            Mismatch::UnknownForum { operation: 2 },
            Mismatch::InvalidCivicName {
                site: patch::Site::Operation(3),
            },
            Mismatch::OfficeHolderNotPerson { operation: 4 },
            Mismatch::OverlappingJurisdiction { operation: 5 },
        ];
        expected.sort();
        assert_eq!(
            reject_owner(&mut kernel, &before, operations(broken)),
            expected
        );
        assert_eq!(kernel.snapshot().unwrap(), before);

        let repaired = vec![
            ComponentOp::OpenOffice {
                institution: Ref::Existing(civic.treasury),
                office: office("steward"),
                delegated: BTreeSet::from([authority_kind(JUDGE_KIND)]),
            },
            ComponentOp::InstallIncumbent {
                institution: Ref::Existing(civic.treasury),
                office: office("steward"),
                incumbent: Ref::Existing(civic.farmer),
            },
            ComponentOp::OpenForum {
                grievance: grievance("eviction"),
                forum: Ref::Existing(civic.treasury),
                standing: over_place(civic.hall),
            },
        ];
        assert!(matches!(
            submit_owner(&mut kernel, &before, operations(repaired)),
            SubmitReceipt::Applied(_)
        ));
    }

    /// A civic operation that changes nothing is not a canonical change.
    #[test]
    fn an_idempotent_civic_operation_is_no_change() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel =
            crate::world::custody_tests::custody_kernel(directory.path(), "Idempotent");
        let (_, civic, _) = civic_world(&mut kernel);
        for operation in [
            grant_to(civic.treasury, LEVY_KIND, over_place(civic.hall)),
            ComponentOp::RevokeAuthority {
                holder: Ref::Existing(civic.treasury),
                grant: grant_of(JUDGE_KIND, over_place(civic.hall)),
            },
            ComponentOp::VacateOffice {
                institution: Ref::Existing(civic.treasury),
                office: office(BAILIFF_OFFICE),
            },
            ComponentOp::OpenForum {
                grievance: grievance(SEIZURE_GRIEVANCE),
                forum: Ref::Existing(civic.treasury),
                standing: over_place(civic.chamber),
            },
            ComponentOp::OpenOffice {
                institution: Ref::Existing(civic.treasury),
                office: office(WARDEN_OFFICE),
                delegated: BTreeSet::from([authority_kind(LEVY_KIND)]),
            },
        ] {
            let before = kernel.snapshot().unwrap();
            assert_eq!(
                reject_owner(&mut kernel, &before, operations(vec![operation])),
                vec![Mismatch::NoOperationEffect { operation: 0 }]
            );
        }
    }

    /// The digest reads what the actor is authorized over and nothing about the
    /// world the actor reaches.
    #[test]
    fn scope_digest_reads_authority_and_delegation_but_not_occupancy() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = crate::world::custody_tests::custody_kernel(directory.path(), "Digest");
        let (topology, civic, _) = civic_world(&mut kernel);
        let scope = |subject_id| DecisionScope { subject_id };
        let digest_of = |kernel: &WorldKernel, subject_id| {
            scope_digest(&kernel.state, scope(subject_id)).unwrap()
        };
        let admit = |kernel: &mut WorldKernel, ops: Vec<ComponentOp>| {
            let before = kernel.snapshot().unwrap();
            submit_owner(kernel, &before, operations(ops));
        };

        // Moving an unrelated subject into and out of the jurisdiction, and
        // opening a forum, leave every digest alone.
        let reeve_before = digest_of(&kernel, civic.reeve);
        let treasury_before = digest_of(&kernel, civic.treasury);
        admit(
            &mut kernel,
            vec![
                ComponentOp::Relocate {
                    subject: Ref::Existing(topology.walker),
                    via: Ref::Existing(topology.ramp),
                },
                ComponentOp::OpenForum {
                    grievance: grievance("eviction"),
                    forum: Ref::Existing(civic.treasury),
                    standing: over_place(civic.hall),
                },
            ],
        );
        assert_eq!(digest_of(&kernel, civic.reeve), reeve_before);
        assert_eq!(digest_of(&kernel, civic.treasury), treasury_before);

        // Installing an office the subject does not hold leaves it alone too.
        admit(
            &mut kernel,
            vec![ComponentOp::InstallIncumbent {
                institution: Ref::Existing(civic.treasury),
                office: office(BAILIFF_OFFICE),
                incumbent: Ref::Existing(civic.farmer),
            }],
        );
        assert_eq!(digest_of(&kernel, civic.reeve), reeve_before);
        assert_eq!(digest_of(&kernel, civic.treasury), treasury_before);

        // Revoking the institution's grant for a lent kind moves the
        // incumbent's digest, because the office copies it in.
        admit(
            &mut kernel,
            vec![ComponentOp::RevokeAuthority {
                holder: Ref::Existing(civic.treasury),
                grant: grant_of(LEVY_KIND, over_place(civic.hall)),
            }],
        );
        assert_ne!(digest_of(&kernel, civic.reeve), reeve_before);
        assert_ne!(digest_of(&kernel, civic.treasury), treasury_before);

        // Granting the actor's own, and narrowing a held office, move it too.
        let reeve_before = digest_of(&kernel, civic.reeve);
        admit(
            &mut kernel,
            vec![grant_to(civic.reeve, LEVY_KIND, over_place(civic.chamber))],
        );
        assert_ne!(digest_of(&kernel, civic.reeve), reeve_before);
        let reeve_before = digest_of(&kernel, civic.reeve);
        admit(
            &mut kernel,
            vec![ComponentOp::OpenOffice {
                institution: Ref::Existing(civic.treasury),
                office: office(WARDEN_OFFICE),
                delegated: BTreeSet::from([authority_kind(COMMAND_KIND)]),
            }],
        );
        assert_ne!(digest_of(&kernel, civic.reeve), reeve_before);
        let reeve_before = digest_of(&kernel, civic.reeve);
        admit(
            &mut kernel,
            vec![ComponentOp::VacateOffice {
                institution: Ref::Existing(civic.treasury),
                office: office(WARDEN_OFFICE),
            }],
        );
        assert_ne!(digest_of(&kernel, civic.reeve), reeve_before);
    }

    /// The snapshot carries a subject's own civic state and no gazetteer: a
    /// forum is visible only to its standing, and no subject sees another's
    /// grants.
    #[test]
    fn a_forum_is_visible_only_to_its_standing() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = crate::world::custody_tests::custody_kernel(directory.path(), "Redress");
        let (_, civic, active) = civic_world(&mut kernel);
        let of = |subject_id: SubjectId| {
            active
                .subjects
                .iter()
                .find(|subject| subject.id == subject_id)
                .expect("a declared subject")
        };

        // The farmer stands in the chamber, which is the forum's standing.
        assert_eq!(
            of(civic.farmer).redress,
            vec![ForumSnapshot {
                grievance: grievance(SEIZURE_GRIEVANCE),
                forum: civic.treasury,
            }]
        );
        assert!(of(civic.outsider).redress.is_empty());

        // Grants are the subject's own; offices held and offices granted are
        // the two ends of one link and never merge.
        assert!(of(civic.reeve).authority.is_empty());
        assert_eq!(of(civic.reeve).offices_held.len(), 1);
        assert!(of(civic.reeve).offices_granted.is_empty());
        assert_eq!(of(civic.treasury).offices_granted.len(), 2);
        assert!(of(civic.treasury).offices_held.is_empty());
        assert!(of(civic.farmer).authority.is_empty());
        assert_eq!(
            of(civic.treasury).authority,
            BTreeSet::from([
                AuthorityGrant {
                    kind: authority_kind(LEVY_KIND),
                    over: AuthorityTarget::PlaceSubtree(civic.hall),
                },
                AuthorityGrant {
                    kind: authority_kind(COMMAND_KIND),
                    over: AuthorityTarget::PlaceSubtree(civic.hall),
                },
            ])
        );
    }

    /// Soul falsification of the candidate authority shadow's reach: a grant
    /// minted in the same patch resolves, but a *destination* declared in the
    /// same patch does not, because the shadow is projected to canonical
    /// referents before the covering predicate runs. A genesis patch therefore
    /// cannot declare a restricted room and walk anyone into it in one batch.
    #[test]
    fn a_same_patch_destination_is_covered_by_no_grant() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = crate::world::custody_tests::custody_kernel(directory.path(), "DraftDoor");
        let topology = admit_topology(&mut kernel);
        let before = kernel.snapshot().unwrap();
        let patch = WorldPatch {
            declarations: vec![
                Declaration::Entity(EntityDeclaration {
                    handle: DraftHandle::new("keep"),
                    label: "The Inner Keep".into(),
                    kind: EntityKind::Place,
                    container: None,
                }),
                Declaration::Route(RouteDeclaration {
                    handle: DraftHandle::new("keepgate"),
                    label: "The Keep Gate".into(),
                    from: Ref::Existing(topology.yard),
                    to: Ref::Draft(DraftHandle::new("keep")),
                    access: AccessKind::Restricted {
                        requires: authority_kind(ADMIT_KIND),
                    },
                    cost: Cost(2),
                }),
            ],
            operations: vec![
                ComponentOp::GrantAuthority {
                    holder: Ref::Existing(topology.walker),
                    grant: AuthorityGrantRef {
                        kind: authority_kind(ADMIT_KIND),
                        over: AuthorityTargetRef::PlaceSubtree(Ref::Draft(DraftHandle::new(
                            "keep",
                        ))),
                    },
                },
                ComponentOp::Relocate {
                    subject: Ref::Existing(topology.walker),
                    via: Ref::Draft(DraftHandle::new("keepgate")),
                },
            ],
            evidence: Vec::new(),
        };
        assert_eq!(
            reject_owner(
                &mut kernel,
                &before,
                CommandBody::AdmitPatch {
                    answers: None,
                    patch
                }
            ),
            vec![Mismatch::RouteAccessRestricted { operation: 1 }],
            "the key covers the door, but the door's far side is still a draft"
        );
    }
}

#[cfg(test)]
mod custody_tests {
    use super::tests::{
        OPENING_BALANCE, TITHE_RECEIPT, activate, admit_custody, admit_topology, auth_principal,
        command, creation, custody_world, operations, opportunity_for, owner, reject_owner,
        speak_entry, submit_owner,
    };
    use super::*;

    pub(super) fn custody_kernel(path: &Path, title: &str) -> WorldKernel {
        WorldKernel::create(
            path.join("world.cc"),
            creation(CommandId::new(), title),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0
    }

    fn holding(kernel: &WorldKernel, holder: SubjectId, resource: EntityId) -> u64 {
        held(&kernel.state, holder, resource)
    }

    /// A forged `PatchAdmitted` never passes the resolver, so `admit_resolved`
    /// re-derives every custody precondition from the committed partitions and
    /// commits nothing when one fails.
    ///
    /// Deviation from the spec's wording, decided here: `ResolvedOp::Transfer`
    /// carries one `qty`, so "a transfer whose debit and credit differ" is
    /// unrepresentable — which is the stronger outcome that shape was chosen
    /// for. The reachable forgery is a transfer the holder cannot cover, and
    /// the conservation equation itself is falsified directly beside it.
    #[test]
    fn a_non_conserving_transfer_effect_does_not_apply() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Forged");
        let (_, custody, _) = custody_world(&mut kernel);
        let before = kernel.state.holdings.clone();

        let mut candidate = kernel.state.clone();
        let forged = WorldEffect::PatchAdmitted {
            resolved: ResolvedPatch {
                subjects: Vec::new(),
                entities: Vec::new(),
                routes: Vec::new(),
                affordances: Vec::new(),
                facts: Vec::new(),
                channels: Vec::new(),
                operations: vec![ResolvedOp::Transfer {
                    from: custody.holder,
                    to: custody.counterparty,
                    resource: custody.tithe,
                    qty: Quantity(OPENING_BALANCE + 1),
                }],
                evidence: Vec::new(),
                scale_intent: None,
            },
            answers: None,
        };
        let error = apply_effect(
            &mut candidate,
            CommandId::issue(),
            &CallerId::Principal(owner()),
            &forged,
        )
        .unwrap_err();
        assert!(matches!(error, KernelError::Invariant(_)));
        assert_eq!(candidate.holdings, before);

        // The conservation equation is the one owner, and it refuses a ledger
        // that does not balance regardless of which operation produced it.
        let unbalanced = BTreeMap::from([(
            custody.tithe,
            LedgerDelta {
                before: 7,
                after: 9,
                admitted: 1,
                ..LedgerDelta::default()
            },
        )]);
        assert_eq!(patch::check_ledger(&unbalanced), Some(custody.tithe));
        let balanced = BTreeMap::from([(
            custody.tithe,
            LedgerDelta {
                before: 7,
                after: 8,
                admitted: 1,
                ..LedgerDelta::default()
            },
        )]);
        assert_eq!(patch::check_ledger(&balanced), None);
    }

    /// Creation of quantity is attributable: an `Admit` whose ref is not in the
    /// patch's own evidence list mints nothing and allocates no ID.
    #[test]
    fn an_admit_without_evidence_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Unevidenced");
        let topology = admit_topology(&mut kernel);
        let speak = speak_entry(&kernel);
        let before = kernel.snapshot().unwrap();
        let commits = kernel.journal.commit_count();

        let mismatches = reject_owner(
            &mut kernel,
            &before,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: vec![
                        Declaration::Entity(EntityDeclaration {
                            handle: DraftHandle::new("tithe"),
                            label: "The Rhythm Tithe".into(),
                            kind: EntityKind::Resource,
                            container: None,
                        }),
                        Declaration::Subject(SubjectDeclaration {
                            handle: DraftHandle::new("clerk"),
                            label: "The Ledger Clerk".into(),
                            kind: SubjectKind::Institution,
                            controller: NewController::OperationalAgent,
                            affordances: BTreeSet::from([speak.clone()]),
                            position: Some(Ref::Existing(topology.yard)),
                        }),
                    ],
                    operations: vec![ComponentOp::Admit {
                        holder: Ref::Draft(DraftHandle::new("clerk")),
                        resource: Ref::Draft(DraftHandle::new("tithe")),
                        qty: Quantity(4),
                        evidence: EvidenceRef::new("receipt:never-listed"),
                    }],
                    evidence: Vec::new(),
                },
            },
        );

        assert_eq!(
            mismatches,
            vec![Mismatch::AdmitWithoutEvidence { operation: 0 }]
        );
        assert!(kernel.state.holdings.is_empty());
        assert_eq!(kernel.journal.commit_count(), commits);
        assert_eq!(kernel.snapshot().unwrap(), before);
    }

    #[test]
    fn a_transfer_moves_the_exact_quantity_and_nothing_else() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Transfer");
        let (_, custody, active) = custody_world(&mut kernel);
        let entities = kernel.state.entities.clone();
        let edges = kernel.state.edges.clone();
        let positions = kernel.state.positions.clone();

        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::Transfer {
                from: Ref::Existing(custody.holder),
                to: Ref::Existing(custody.counterparty),
                resource: Ref::Existing(custody.tithe),
                qty: Quantity(3),
            }]),
        );

        assert_eq!(holding(&kernel, custody.holder, custody.tithe), 4);
        assert_eq!(holding(&kernel, custody.counterparty, custody.tithe), 3);
        assert_eq!(resource_total(&kernel.state, custody.tithe), 7);
        assert_eq!(kernel.state.entities, entities);
        assert_eq!(kernel.state.edges, edges);
        assert_eq!(kernel.state.positions, positions);
    }

    /// Absence is zero at both levels, so emptying a holding removes the
    /// resource key and emptying a holder removes the holder key.
    #[test]
    fn consume_reduces_the_holding_and_emptying_removes_the_key() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Consume");
        let (_, custody, active) = custody_world(&mut kernel);

        let after_three = submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::Consume {
                holder: Ref::Existing(custody.holder),
                resource: Ref::Existing(custody.tithe),
                qty: Quantity(3),
            }]),
        );
        assert!(matches!(after_three, SubmitReceipt::Applied(_)));
        assert_eq!(holding(&kernel, custody.holder, custody.tithe), 4);

        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::Consume {
                holder: Ref::Existing(custody.holder),
                resource: Ref::Existing(custody.tithe),
                qty: Quantity(4),
            }]),
        );
        assert!(!kernel.state.holdings.contains_key(&custody.holder));
        let subject = kernel
            .snapshot()
            .unwrap()
            .subjects
            .into_iter()
            .find(|subject| subject.id == custody.holder)
            .expect("the holder is in the snapshot");
        assert!(subject.holdings.is_empty());
    }

    #[test]
    fn a_transform_conserves_one_for_one() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Transform");
        let (_, custody, active) = custody_world(&mut kernel);

        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::Transform {
                holder: Ref::Existing(custody.holder),
                from_resource: Ref::Existing(custody.tithe),
                into_resource: Ref::Existing(custody.ingot),
                qty: Quantity(5),
            }]),
        );

        assert_eq!(holding(&kernel, custody.holder, custody.tithe), 2);
        assert_eq!(holding(&kernel, custody.holder, custody.ingot), 5);
        assert_eq!(resource_total(&kernel.state, custody.tithe), 2);
        assert_eq!(resource_total(&kernel.state, custody.ingot), 5);
    }

    #[test]
    fn spending_more_than_held_is_rejected_with_insufficient_custody() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Insufficient");
        let (_, custody, _) = custody_world(&mut kernel);
        let too_much = Quantity(OPENING_BALANCE + 1);
        let attempts = [
            ComponentOp::Transfer {
                from: Ref::Existing(custody.holder),
                to: Ref::Existing(custody.counterparty),
                resource: Ref::Existing(custody.tithe),
                qty: too_much,
            },
            ComponentOp::Transform {
                holder: Ref::Existing(custody.holder),
                from_resource: Ref::Existing(custody.tithe),
                into_resource: Ref::Existing(custody.ingot),
                qty: too_much,
            },
            ComponentOp::Consume {
                holder: Ref::Existing(custody.holder),
                resource: Ref::Existing(custody.tithe),
                qty: too_much,
            },
        ];

        for attempt in attempts {
            let snapshot = kernel.snapshot().unwrap();
            let commits = kernel.journal.commit_count();
            let mismatches = reject_owner(&mut kernel, &snapshot, operations(vec![attempt]));
            assert_eq!(
                mismatches,
                vec![Mismatch::InsufficientCustody { operation: 0 }]
            );
            assert_eq!(kernel.journal.commit_count(), commits);
            assert_eq!(kernel.snapshot().unwrap(), snapshot);
        }
    }

    /// A holder with no entry holds zero: there is no second name for it and no
    /// zero entry anywhere in the snapshot.
    #[test]
    fn an_absent_holding_is_zero() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Absent");
        let (_, custody, active) = custody_world(&mut kernel);

        let mismatches = reject_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::Consume {
                holder: Ref::Existing(custody.counterparty),
                resource: Ref::Existing(custody.tithe),
                qty: Quantity(1),
            }]),
        );
        assert_eq!(
            mismatches,
            vec![Mismatch::InsufficientCustody { operation: 0 }]
        );
        let counterparty = kernel
            .snapshot()
            .unwrap()
            .subjects
            .into_iter()
            .find(|subject| subject.id == custody.counterparty)
            .expect("the counterparty is in the snapshot");
        assert!(counterparty.holdings.is_empty());
    }

    #[test]
    fn a_zero_quantity_operation_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Zero");
        let topology = admit_topology(&mut kernel);
        let custody = admit_custody(&mut kernel, &topology);

        // `Admit` is Draft-only, because Active refuses a patch carrying
        // evidence, so its zero case is proven before activation.
        let draft = kernel.snapshot().unwrap();
        let mismatches = reject_owner(
            &mut kernel,
            &draft,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: Vec::new(),
                    operations: vec![ComponentOp::Admit {
                        holder: Ref::Existing(custody.holder),
                        resource: Ref::Existing(custody.tithe),
                        qty: Quantity(0),
                        evidence: EvidenceRef::new(TITHE_RECEIPT),
                    }],
                    evidence: vec![EvidenceRef::new(TITHE_RECEIPT)],
                },
            },
        );
        assert_eq!(mismatches, vec![Mismatch::ZeroQuantity { operation: 0 }]);

        let active = activate(&mut kernel);
        for attempt in [
            ComponentOp::Transfer {
                from: Ref::Existing(custody.holder),
                to: Ref::Existing(custody.counterparty),
                resource: Ref::Existing(custody.tithe),
                qty: Quantity(0),
            },
            ComponentOp::Consume {
                holder: Ref::Existing(custody.holder),
                resource: Ref::Existing(custody.tithe),
                qty: Quantity(0),
            },
        ] {
            let snapshot = kernel.snapshot().unwrap();
            let mismatches = reject_owner(&mut kernel, &snapshot, operations(vec![attempt]));
            assert_eq!(mismatches, vec![Mismatch::ZeroQuantity { operation: 0 }]);
        }
        assert_eq!(kernel.snapshot().unwrap(), active);
    }

    #[test]
    fn dependency_bind_and_release_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Dependency");
        let (_, custody, active) = custody_world(&mut kernel);
        let before = kernel.state.dependencies.clone();

        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::Bind {
                subject: Ref::Existing(custody.holder),
                target: DependencyRef::Resource(Ref::Existing(custody.tithe)),
            }]),
        );
        assert_eq!(
            kernel.state.dependencies.get(&custody.holder),
            Some(&BTreeSet::from([DependencyTarget::Resource(custody.tithe)]))
        );

        let bound = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &bound,
            operations(vec![ComponentOp::Release {
                subject: Ref::Existing(custody.holder),
                target: DependencyRef::Resource(Ref::Existing(custody.tithe)),
            }]),
        );
        assert_eq!(kernel.state.dependencies, before);
        assert!(!kernel.state.dependencies.contains_key(&custody.holder));
    }

    /// A bind that changes nothing is `NoOperationEffect`, the name pass 2
    /// already owns; a subject bound to itself is `SelfDependency`.
    #[test]
    fn a_duplicate_dependency_bind_is_rejected_as_no_effect() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Duplicate");
        let (_, custody, active) = custody_world(&mut kernel);

        let bind = ComponentOp::Bind {
            subject: Ref::Existing(custody.holder),
            target: DependencyRef::Resource(Ref::Existing(custody.tithe)),
        };
        submit_owner(&mut kernel, &active, operations(vec![bind.clone()]));

        let bound = kernel.snapshot().unwrap();
        assert_eq!(
            reject_owner(&mut kernel, &bound, operations(vec![bind])),
            vec![Mismatch::NoOperationEffect { operation: 0 }]
        );
        assert_eq!(
            reject_owner(
                &mut kernel,
                &bound,
                operations(vec![ComponentOp::Release {
                    subject: Ref::Existing(custody.holder),
                    target: DependencyRef::Resource(Ref::Existing(custody.ingot)),
                }]),
            ),
            vec![Mismatch::NoOperationEffect { operation: 0 }]
        );
        assert_eq!(
            reject_owner(
                &mut kernel,
                &bound,
                operations(vec![ComponentOp::Bind {
                    subject: Ref::Existing(custody.holder),
                    target: DependencyRef::Subject(Ref::Existing(custody.holder)),
                }]),
            ),
            vec![Mismatch::SelfDependency { operation: 0 }]
        );
    }

    /// Pass 3 owns the representation of a dependency, not its consequence. A
    /// dependency on a closed route commits and shows up in the subject's
    /// components; nothing rejects it and nothing fires from it.
    #[test]
    fn a_dependency_on_a_closed_route_is_representable_and_visible() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Closed");
        let (topology, custody, active) = custody_world(&mut kernel);
        assert!(!kernel.state.edges[&topology.shutter].is_open());

        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::Bind {
                subject: Ref::Existing(custody.holder),
                target: DependencyRef::Route(Ref::Existing(topology.shutter)),
            }]),
        );

        let subject = kernel
            .snapshot()
            .unwrap()
            .subjects
            .into_iter()
            .find(|subject| subject.id == custody.holder)
            .expect("the holder is in the snapshot");
        assert_eq!(
            subject.dependencies,
            BTreeSet::from([DependencyTarget::Route(topology.shutter)])
        );
    }

    /// The digest reads the acting subject's own holdings and dependencies, and
    /// only its own: a counterparty's churn does not move it.
    #[test]
    fn scope_digest_reads_holdings_and_dependencies() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "ScopeCustody");
        let topology = admit_topology(&mut kernel);
        let custody = admit_custody(&mut kernel, &topology);
        activate(&mut kernel);
        let scope = DecisionScope {
            subject_id: custody.holder,
        };
        let base = scope_digest(&kernel.state, scope).unwrap();

        let mut spent = kernel.state.clone();
        set_held(&mut spent, custody.holder, custody.tithe, 1);
        assert_ne!(scope_digest(&spent, scope).unwrap(), base);

        let mut gained = kernel.state.clone();
        set_held(&mut gained, custody.holder, custody.ingot, 1);
        assert_ne!(scope_digest(&gained, scope).unwrap(), base);

        let mut bound = kernel.state.clone();
        bound
            .dependencies
            .entry(custody.holder)
            .or_default()
            .insert(DependencyTarget::Resource(custody.tithe));
        assert_ne!(scope_digest(&bound, scope).unwrap(), base);

        let mut elsewhere = kernel.state.clone();
        set_held(&mut elsewhere, custody.counterparty, custody.tithe, 9);
        elsewhere
            .dependencies
            .entry(custody.counterparty)
            .or_default()
            .insert(DependencyTarget::Resource(custody.ingot));
        assert_eq!(scope_digest(&elsewhere, scope).unwrap(), base);
    }

    /// A proposal bound while its subject held seven must not commit after
    /// someone took six. Custody churn rejects more bound proposals than route
    /// churn did; that is the binding working.
    #[test]
    fn a_spend_bound_to_a_stale_balance_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Stale");
        let (_, custody, active) = custody_world(&mut kernel);
        let bound = opportunity_for(&active, custody.holder);

        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::Transfer {
                from: Ref::Existing(custody.holder),
                to: Ref::Existing(custody.counterparty),
                resource: Ref::Existing(custody.tithe),
                qty: Quantity(6),
            }]),
        );

        let moved = kernel.snapshot().unwrap();
        let commits = kernel.journal.commit_count();
        let error = kernel
            .submit(
                command(
                    &moved,
                    CommandId::new(),
                    CallerId::Controller(bound.controller_id),
                    CommandBody::ExerciseDecision {
                        opportunity: bound.clone(),
                        invocation: DecisionInvocation {
                            affordance: bound.affordance_ids[0],
                            bindings: Vec::new(),
                            proposed: Vec::new(),
                            speech: Some(Statement::new("The tithe is short.").unwrap()),
                        },
                    },
                ),
                &AuthenticatedCaller::fixture(CallerId::Controller(bound.controller_id)),
            )
            .unwrap_err();

        assert!(matches!(error, KernelError::ScopeChanged { .. }));
        assert_eq!(kernel.journal.commit_count(), commits);
        assert_eq!(kernel.snapshot().unwrap(), moved);
    }

    /// Soul, proof 4: the ledger equation stated as arithmetic over the
    /// committed partitions, across one patch that exercises every conserving
    /// operation at once. Per resource, `after == before + admitted + gained -
    /// consumed - spent`, and no third resource moves.
    #[test]
    fn soul_a_mixed_patch_conserves_every_resource_it_touches() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "MixedLedger");
        let (_, custody, active) = custody_world(&mut kernel);

        // The opening balance was created by `Admit` alone: before is zero,
        // admitted is the balance, and the total equals the sum of the two.
        assert_eq!(
            resource_total(&kernel.state, custody.tithe),
            u128::from(OPENING_BALANCE)
        );
        assert_eq!(resource_total(&kernel.state, custody.ingot), 0);

        let tithe_before = resource_total(&kernel.state, custody.tithe);
        let ingot_before = resource_total(&kernel.state, custody.ingot);

        let receipt = submit_owner(
            &mut kernel,
            &active,
            operations(vec![
                ComponentOp::Transfer {
                    from: Ref::Existing(custody.holder),
                    to: Ref::Existing(custody.counterparty),
                    resource: Ref::Existing(custody.tithe),
                    qty: Quantity(3),
                },
                ComponentOp::Transform {
                    holder: Ref::Existing(custody.holder),
                    from_resource: Ref::Existing(custody.tithe),
                    into_resource: Ref::Existing(custody.ingot),
                    qty: Quantity(2),
                },
                ComponentOp::Consume {
                    holder: Ref::Existing(custody.counterparty),
                    resource: Ref::Existing(custody.tithe),
                    qty: Quantity(1),
                },
            ]),
        );
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));

        // Transfer contributes to no term; transform contributes one spent and
        // one gained; consume contributes one consumed. Nothing was admitted.
        assert_eq!(
            resource_total(&kernel.state, custody.tithe),
            tithe_before - 1 - 2
        );
        assert_eq!(
            resource_total(&kernel.state, custody.ingot),
            ingot_before + 2
        );
        assert_eq!(holding(&kernel, custody.holder, custody.tithe), 2);
        assert_eq!(holding(&kernel, custody.holder, custody.ingot), 2);
        assert_eq!(holding(&kernel, custody.counterparty, custody.tithe), 2);
        // Absence is zero: the counterparty never held an ingot, so there is no
        // row for one rather than a row holding nothing.
        assert!(!kernel.state.holdings[&custody.counterparty].contains_key(&custody.ingot));
    }

    /// Soul: is the apply-side `check_ledger` reachable?
    ///
    /// A forged `PatchAdmitted` can carry a *sequence* whose second operation
    /// exceeds what the first left. It dies at `apply_operation`'s own
    /// re-derived precondition, one operation early, so the ledger never speaks.
    /// A sequence in which every operation applies cannot break the equation,
    /// because every `ResolvedOp` arm debits and credits the same `qty` it
    /// records: the apply-side call is unreachable from the current closed set.
    #[test]
    fn soul_a_forged_sequence_dies_at_the_per_operation_check_not_the_ledger() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "ForgedSequence");
        let (_, custody, _) = custody_world(&mut kernel);
        let before = kernel.state.holdings.clone();

        let forge = |operations: Vec<ResolvedOp>| WorldEffect::PatchAdmitted {
            resolved: ResolvedPatch {
                subjects: Vec::new(),
                entities: Vec::new(),
                routes: Vec::new(),
                affordances: Vec::new(),
                facts: Vec::new(),
                channels: Vec::new(),
                operations,
                evidence: Vec::new(),
                scale_intent: None,
            },
            answers: None,
        };
        let transfer = |qty: u64| ResolvedOp::Transfer {
            from: custody.holder,
            to: custody.counterparty,
            resource: custody.tithe,
            qty: Quantity(qty),
        };

        // Two transfers of five out of a balance of seven: the first leaves two,
        // the second cannot cover five.
        let mut candidate = kernel.state.clone();
        let error = apply_effect(
            &mut candidate,
            CommandId::issue(),
            &CallerId::Principal(owner()),
            &forge(vec![transfer(5), transfer(5)]),
        )
        .unwrap_err();
        assert!(
            matches!(&error, KernelError::Invariant(message) if message == "holder does not hold enough"),
            "the per-operation precondition speaks first, not the ledger: {error:?}"
        );
        // The candidate is a throwaway clone; the live state never moved.
        assert_eq!(kernel.state.holdings, before);

        // Every operation applying means the equation balances, so the same
        // forged lane commits a conserving sequence with no ledger complaint.
        let mut conserving = kernel.state.clone();
        apply_effect(
            &mut conserving,
            CommandId::issue(),
            &CallerId::Principal(owner()),
            &forge(vec![
                transfer(5),
                ResolvedOp::Transfer {
                    from: custody.counterparty,
                    to: custody.holder,
                    resource: custody.tithe,
                    qty: Quantity(5),
                },
            ]),
        )
        .expect("a conserving forged sequence applies");
        assert_eq!(conserving.holdings, before);

        // The degenerate shapes a forgery could otherwise use to mint or burn
        // are refused by the arms themselves, not by the equation.
        for degenerate in [
            ResolvedOp::Transfer {
                from: custody.holder,
                to: custody.holder,
                resource: custody.tithe,
                qty: Quantity(1),
            },
            ResolvedOp::Transform {
                holder: custody.holder,
                from_resource: custody.tithe,
                into_resource: custody.tithe,
                qty: Quantity(1),
            },
        ] {
            let mut candidate = kernel.state.clone();
            let error = apply_effect(
                &mut candidate,
                CommandId::issue(),
                &CallerId::Principal(owner()),
                &forge(vec![degenerate]),
            )
            .unwrap_err();
            assert!(
                matches!(&error, KernelError::Invariant(message) if message == "custody operation moves nothing"),
                "unexpected refusal: {error:?}"
            );
        }

        // An `Admit` in a forged effect still needs its receipt in the same
        // patch, so the apply lane mints no unattributable quantity either.
        let mut unevidenced = kernel.state.clone();
        let error = apply_effect(
            &mut unevidenced,
            CommandId::issue(),
            &CallerId::Principal(owner()),
            &forge(vec![ResolvedOp::Admit {
                holder: custody.holder,
                resource: custody.tithe,
                qty: Quantity(4),
                evidence: EvidenceRef::new(TITHE_RECEIPT),
            }]),
        )
        .unwrap_err();
        assert!(
            matches!(&error, KernelError::Invariant(message) if message == "admitted quantity cites no evidence in its patch"),
            "unexpected refusal: {error:?}"
        );
        assert_eq!(kernel.state.holdings, before);
    }

    /// Soul: `QuantityOverflow` is reachable, and the arithmetic that reaches it
    /// is `checked_add` rather than a silent clamp. Two admissions in one patch
    /// whose sum exceeds `u64::MAX` mint nothing and allocate no ID.
    #[test]
    fn soul_a_holding_cannot_overflow_silently() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "Overflow");
        let topology = admit_topology(&mut kernel);
        let speak = speak_entry(&kernel);
        let before = kernel.snapshot().unwrap();
        let commits = kernel.journal.commit_count();

        let admit = |qty: u64| ComponentOp::Admit {
            holder: Ref::Draft(DraftHandle::new("clerk")),
            resource: Ref::Draft(DraftHandle::new("tithe")),
            qty: Quantity(qty),
            evidence: EvidenceRef::new(TITHE_RECEIPT),
        };
        let mismatches = reject_owner(
            &mut kernel,
            &before,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: vec![
                        Declaration::Entity(EntityDeclaration {
                            handle: DraftHandle::new("tithe"),
                            label: "The Rhythm Tithe".into(),
                            kind: EntityKind::Resource,
                            container: None,
                        }),
                        Declaration::Subject(SubjectDeclaration {
                            handle: DraftHandle::new("clerk"),
                            label: "The Ledger Clerk".into(),
                            kind: SubjectKind::Institution,
                            controller: NewController::OperationalAgent,
                            affordances: BTreeSet::from([speak.clone()]),
                            position: Some(Ref::Existing(topology.yard)),
                        }),
                    ],
                    operations: vec![admit(u64::MAX), admit(1)],
                    evidence: vec![EvidenceRef::new(TITHE_RECEIPT)],
                },
            },
        );

        assert_eq!(
            mismatches,
            vec![Mismatch::QuantityOverflow { operation: 1 }],
            "the overflow is the complete verdict: no clamp, no second name"
        );
        assert!(kernel.state.holdings.is_empty());
        assert_eq!(kernel.journal.commit_count(), commits);
        assert_eq!(kernel.snapshot().unwrap(), before);
    }

    /// Soul, scope: a transfer changes the *receiver's* components too, so the
    /// counterparty's in-flight proposal is refused with `ScopeChanged` while an
    /// unrelated subject's bound proposal still commits at the later revision.
    #[test]
    fn soul_an_incoming_transfer_changes_only_the_counterpartys_scope() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "IncomingScope");
        let (topology, custody, active) = custody_world(&mut kernel);
        let receiver = opportunity_for(&active, custody.counterparty);
        let bystander = opportunity_for(&active, topology.walker);

        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::Transfer {
                from: Ref::Existing(custody.holder),
                to: Ref::Existing(custody.counterparty),
                resource: Ref::Existing(custody.tithe),
                qty: Quantity(3),
            }]),
        );
        let moved = kernel.snapshot().unwrap();

        let exercise = |opportunity: &DecisionOpportunity| CommandBody::ExerciseDecision {
            opportunity: opportunity.clone(),
            invocation: DecisionInvocation {
                affordance: opportunity.affordance_ids[0],
                bindings: Vec::new(),
                proposed: Vec::new(),
                speech: Some(Statement::new("Counted before the tithe arrived.").unwrap()),
            },
        };

        let error = kernel
            .submit(
                command(
                    &moved,
                    CommandId::new(),
                    CallerId::Controller(receiver.controller_id),
                    exercise(&receiver),
                ),
                &AuthenticatedCaller::fixture(CallerId::Controller(receiver.controller_id)),
            )
            .unwrap_err();
        assert!(
            matches!(error, KernelError::ScopeChanged { .. }),
            "the receiver's holdings moved under its bound proposal: {error:?}"
        );

        let receipt = kernel
            .submit(
                command(
                    &moved,
                    CommandId::new(),
                    CallerId::Controller(bystander.controller_id),
                    exercise(&bystander),
                ),
                &AuthenticatedCaller::fixture(CallerId::Controller(bystander.controller_id)),
            )
            .expect("an unrelated subject's bound proposal is untouched by the transfer");
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
    }
}

/// Pass 6: `Knowledge`, `Channel`, `Fact` standing, and the non-leakage rule.
/// A subject perceives a speech act if and only if it holds `Knowledge` of that
/// act's fact — the knowledge partition is the sole owner of who-heard-what.
#[cfg(test)]
mod knowledge_tests {
    use super::patch::{RefName, Site};
    use super::tests::{
        FLOOD_EVIDENCE, FLOOD_STATEMENT, Speech, auth_principal, command, creation, operations,
        opportunity_for, owner, reject_owner, speech_world, spoken, submit_owner,
    };
    use super::*;
    use std::path::Path;

    fn speech_kernel(path: &Path, title: &str) -> (WorldKernel, Speech, WorldSnapshot) {
        let mut kernel = WorldKernel::create(
            path.join("world.cc"),
            creation(CommandId::new(), title),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let (speech, active) = speech_world(&mut kernel);
        (kernel, speech, active)
    }

    fn utter(
        kernel: &mut WorldKernel,
        snapshot: &WorldSnapshot,
        actor: SubjectId,
        invocation: DecisionInvocation,
    ) -> Result<SubmitReceipt, KernelError> {
        let opportunity = opportunity_for(snapshot, actor);
        let caller = CallerId::Controller(opportunity.controller_id);
        kernel.submit(
            command(
                snapshot,
                CommandId::new(),
                caller.clone(),
                CommandBody::ExerciseDecision {
                    opportunity,
                    invocation,
                },
            ),
            &AuthenticatedCaller::fixture(caller),
        )
    }

    fn say(affordance: AffordanceId, bindings: Vec<RoleBinding>, text: &str) -> DecisionInvocation {
        DecisionInvocation {
            affordance,
            bindings,
            proposed: Vec::new(),
            speech: Some(Statement::new(text).unwrap()),
        }
    }

    fn binding(role: &str, target: Target) -> RoleBinding {
        RoleBinding {
            role: Role(role.into()),
            target,
        }
    }

    fn rejected(result: Result<SubmitReceipt, KernelError>) -> Vec<ActionMismatch> {
        match result {
            Err(KernelError::ActionRejected(mismatches)) => mismatches,
            other => panic!("expected a rejected invocation, got {other:?}"),
        }
    }

    fn knows(kernel: &WorldKernel, subject: SubjectId, fact: EntityId) -> Option<Knowledge> {
        kernel
            .state
            .knowledge
            .get(&subject)
            .and_then(|held| held.get(&fact))
            .copied()
    }

    fn acquire(subject: SubjectId, fact: EntityId, source: AuthoredSource) -> ComponentOp {
        ComponentOp::AcquireKnowledge {
            subject: Ref::Existing(subject),
            fact: Ref::Existing(fact),
            source,
            confidence: Confidence::Certain,
        }
    }

    fn minted_claim(kernel: &WorldKernel) -> EntityId {
        kernel
            .state
            .events
            .last()
            .expect("the committed event")
            .speech
            .expect("a speech act names its claim")
    }

    /// Verification 5, first half: a patch adds knowledge only by citing an
    /// accessible fact, and a rejected citation allocates nothing.
    #[test]
    fn knowledge_citing_an_inaccessible_fact_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "Inaccessible");
        let before = kernel.state.clone();

        assert_eq!(
            reject_owner(
                &mut kernel,
                &active,
                operations(vec![acquire(
                    speech.listener,
                    EntityId::issue(),
                    AuthoredSource::Witnessed
                )]),
            ),
            vec![Mismatch::UnknownCanonical {
                site: Site::Operation(0),
                expected: RefKind::Entity(EntityKind::Fact),
            }]
        );
        // A live referent of the wrong kind names the kind it actually is.
        assert_eq!(
            reject_owner(
                &mut kernel,
                &active,
                operations(vec![acquire(
                    speech.listener,
                    speech.hall,
                    AuthoredSource::Witnessed
                )]),
            ),
            vec![Mismatch::WrongKind {
                site: Site::Operation(0),
                referent: RefName::Entity(Ref::Existing(speech.hall)),
                expected: RefKind::Entity(EntityKind::Fact),
                actual: RefKind::Entity(EntityKind::Place),
            }]
        );
        assert_eq!(kernel.state, before);

        submit_owner(
            &mut kernel,
            &active,
            operations(vec![acquire(
                speech.listener,
                speech.flood,
                AuthoredSource::Witnessed,
            )]),
        );
        assert_eq!(
            knows(&kernel, speech.listener, speech.flood).map(|held| held.source),
            Some(KnowledgeSource::Witnessed)
        );
    }

    /// Standing is what a fact *is*, so both halves are checked at declaration:
    /// canon needs its receipt in the same patch, a claim needs a live asserter.
    #[test]
    fn a_canonical_fact_needs_evidence_and_a_claim_needs_an_asserter() {
        let directory = tempfile::tempdir().unwrap();
        let mut draft = WorldKernel::create(
            directory.path().join("world.cc"),
            creation(CommandId::new(), "Standing"),
            &auth_principal(owner()),
        )
        .expect("a draft world")
        .0;
        let start = draft.snapshot().unwrap();
        let declare =
            |standing: FactStandingRef, evidence: Vec<EvidenceRef>| CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: vec![Declaration::Fact(FactDeclaration {
                        handle: DraftHandle::new("rumour"),
                        label: "a rumour".into(),
                        statement: Statement::new("The reeve took two tithes.").unwrap(),
                        standing,
                    })],
                    operations: Vec::new(),
                    evidence,
                },
            };

        assert_eq!(
            reject_owner(
                &mut draft,
                &start,
                declare(
                    FactStandingRef::Canonical {
                        evidence: EvidenceRef::new("vault:absent"),
                    },
                    Vec::new(),
                ),
            ),
            vec![Mismatch::FactWithoutEvidence {
                handle: DraftHandle::new("rumour"),
            }]
        );
        assert_eq!(
            reject_owner(
                &mut draft,
                &start,
                declare(
                    FactStandingRef::Claimed {
                        by: Ref::Draft(DraftHandle::new("nobody")),
                    },
                    Vec::new(),
                ),
            ),
            vec![Mismatch::UnresolvedDraft {
                site: Site::Declaration(DraftHandle::new("rumour")),
                referent: DraftHandle::new("nobody"),
                expected: RefKind::Subject(None),
            }]
        );
        assert!(draft.state.facts.is_empty());

        submit_owner(
            &mut draft,
            &start,
            declare(
                FactStandingRef::Canonical {
                    evidence: EvidenceRef::new("vault:rumour"),
                },
                vec![EvidenceRef::new("vault:rumour")],
            ),
        );
        assert_eq!(draft.state.facts.len(), 1);
        // The bijection holds in both directions.
        let fact = *draft.state.facts.keys().next().unwrap();
        assert_eq!(draft.state.entities[&fact].kind, EntityKind::Fact);
    }

    /// A receipt vouches for canon. It cannot vouch for an assertion the kernel
    /// never evaluated.
    #[test]
    fn evidenced_knowledge_of_a_claim_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "Evidenced");
        utter(
            &mut kernel,
            &active,
            speech.speaker,
            say(
                speech.proclaim,
                vec![binding("channel", Target::Entity(speech.horn))],
                "The gate was left open.",
            ),
        )
        .expect("the proclamation commits");
        let claim = minted_claim(&kernel);
        let active = kernel.snapshot().unwrap();

        assert_eq!(
            reject_owner(
                &mut kernel,
                &active,
                operations(vec![acquire(
                    speech.stranger,
                    claim,
                    AuthoredSource::Evidenced
                )]),
            ),
            vec![Mismatch::EvidencedKnowledgeOfClaim { operation: 0 }]
        );
        submit_owner(
            &mut kernel,
            &active,
            operations(vec![acquire(
                speech.stranger,
                speech.flood,
                AuthoredSource::Evidenced,
            )]),
        );
        assert_eq!(
            knows(&kernel, speech.stranger, speech.flood).map(|held| held.source),
            Some(KnowledgeSource::Evidenced)
        );
        assert_eq!(
            kernel.state.facts[&speech.flood].standing,
            FactStanding::Canonical {
                evidence: EvidenceRef::new(FLOOD_EVIDENCE),
            }
        );
    }

    /// The fan-out is the audience, less the speaker: co-located speech fills
    /// the exact room, and the utterance lands in `facts` and nowhere else.
    #[test]
    fn speak_fans_out_to_the_audience_and_not_beyond() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "FanOut");
        utter(
            &mut kernel,
            &active,
            speech.speaker,
            say(
                speech.whisper,
                vec![binding("target", Target::Subject(speech.listener))],
                "Mind the hinge.",
            ),
        )
        .expect("the whisper commits");

        let event = kernel.state.events.last().expect("the committed event");
        let claim = event.speech.expect("a speech act names its claim");
        assert_eq!(spoken(&kernel.state, event), Some("Mind the hinge."));
        assert_eq!(
            knows(&kernel, speech.listener, claim),
            Some(Knowledge {
                confidence: Confidence::Believed,
                source: KnowledgeSource::Told {
                    by: speech.speaker,
                    via: None,
                },
            })
        );
        // The yard is inside the hall, but a voice fills a room and not the
        // district containing it. The speaker's own claim is not implied.
        for outside in [speech.bystander, speech.stranger, speech.speaker] {
            assert_eq!(knows(&kernel, outside, claim), None);
        }
        assert_eq!(
            kernel.state.facts[&claim].standing,
            FactStanding::Claimed { by: speech.speaker }
        );
        // The utterance has exactly one home: a label is a name, not a
        // transcript.
        assert_ne!(kernel.state.entities[&claim].label, "Mind the hinge.");
    }

    /// Co-location is the exact place; a channel's `Reach::Place` is the
    /// subtree, through the same containment walk a jurisdiction uses.
    #[test]
    fn co_location_reach_is_the_exact_place_and_a_channel_place_is_the_subtree() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "Reach");
        utter(
            &mut kernel,
            &active,
            speech.speaker,
            say(
                speech.proclaim,
                vec![binding("channel", Target::Entity(speech.horn))],
                "The horn carries.",
            ),
        )
        .expect("the proclamation commits");
        let broadcast = minted_claim(&kernel);
        assert!(knows(&kernel, speech.listener, broadcast).is_some());
        assert!(knows(&kernel, speech.bystander, broadcast).is_some());
        assert_eq!(knows(&kernel, speech.stranger, broadcast), None);

        // The same speaker's voice reaches only the hall.
        let active = kernel.snapshot().unwrap();
        utter(
            &mut kernel,
            &active,
            speech.speaker,
            say(
                speech.whisper,
                vec![binding("target", Target::Subject(speech.listener))],
                "A voice does not.",
            ),
        )
        .expect("the whisper commits");
        let voiced = minted_claim(&kernel);
        assert!(knows(&kernel, speech.listener, voiced).is_some());
        assert_eq!(knows(&kernel, speech.bystander, voiced), None);

        // Moving the bystander up the stair puts it in both audiences.
        let active = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::Relocate {
                subject: Ref::Existing(speech.bystander),
                via: Ref::Existing(speech.stair),
            }]),
        );
        let active = kernel.snapshot().unwrap();
        utter(
            &mut kernel,
            &active,
            speech.speaker,
            say(
                speech.whisper,
                vec![binding("target", Target::Subject(speech.bystander))],
                "Now you hear it.",
            ),
        )
        .expect("the whisper commits");
        assert!(knows(&kernel, speech.bystander, minted_claim(&kernel)).is_some());
    }

    /// The delta of a telling is a property of the world, not a defect in the
    /// proposal: speaking alone commits the claim and the event and lands
    /// nothing.
    #[test]
    fn speaking_into_an_empty_room_commits_the_claim_and_lands_nothing() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "EmptyRoom");
        let knowledge_before = kernel.state.knowledge.clone();
        utter(
            &mut kernel,
            &active,
            speech.bystander,
            say(
                speech.whisper,
                vec![binding("target", Target::Subject(speech.bystander))],
                "Nobody is here.",
            ),
        )
        .expect("speech into an empty room commits");

        assert_eq!(kernel.state.knowledge, knowledge_before);
        assert_eq!(kernel.state.events.len(), 1);
        assert_eq!(
            spoken(&kernel.state, kernel.state.events.last().unwrap()),
            Some("Nobody is here.")
        );
        assert_eq!(
            kernel
                .state
                .facts
                .values()
                .filter(|record| matches!(record.standing, FactStanding::Claimed { .. }))
                .count(),
            1
        );
    }

    /// The horn belongs to the temple: its controller may speak on it whether
    /// or not it stands inside its reach, and removing the controller flips the
    /// same invocation to exactly one named failure.
    #[test]
    fn a_channel_controller_may_speak_outside_its_own_reach() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "Controller");
        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::SetReach {
                channel: Ref::Existing(speech.horn),
                reach: ReachRef::Subjects(BTreeSet::from([Ref::Existing(speech.listener)])),
            }]),
        );
        let proclaim = || {
            say(
                speech.proclaim,
                vec![binding("channel", Target::Entity(speech.horn))],
                "The horn still carries.",
            )
        };
        let active = kernel.snapshot().unwrap();
        utter(&mut kernel, &active, speech.speaker, proclaim())
            .expect("the controller may speak on its own channel");

        let active = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::SetController {
                channel: Ref::Existing(speech.horn),
                controller: None,
            }]),
        );
        let active = kernel.snapshot().unwrap();
        assert_eq!(
            rejected(utter(&mut kernel, &active, speech.speaker, proclaim())),
            vec![ActionMismatch::NoAudience { precondition: 0 }]
        );
    }

    /// Addressing does not narrow the audience, and a target outside it names
    /// itself. A target that walked in mid-flight commits, and the whole
    /// audience receives the telling, not only the addressed subject.
    #[test]
    fn an_addressed_subject_outside_the_audience_names_itself() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "Addressing");
        let whisper = || {
            say(
                speech.whisper,
                vec![binding("target", Target::Subject(speech.bystander))],
                "Come inside.",
            )
        };
        assert_eq!(
            rejected(utter(&mut kernel, &active, speech.speaker, whisper())),
            vec![ActionMismatch::CannotReach { precondition: 0 }]
        );

        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::Relocate {
                subject: Ref::Existing(speech.bystander),
                via: Ref::Existing(speech.stair),
            }]),
        );
        let active = kernel.snapshot().unwrap();
        utter(&mut kernel, &active, speech.speaker, whisper())
            .expect("the addressed subject is now in the room");
        let claim = minted_claim(&kernel);
        assert!(knows(&kernel, speech.bystander, claim).is_some());
        assert!(
            knows(&kernel, speech.listener, claim).is_some(),
            "speaking to someone in a room is heard by the room"
        );
    }

    /// `Knows` compares confidence with one `>=`, and `Forget` removes the key
    /// rather than storing a zero.
    #[test]
    fn knows_fails_below_its_confidence_and_forget_removes() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "Knows");
        let recant = || DecisionInvocation {
            affordance: speech.recant,
            bindings: vec![binding("fact", Target::Entity(speech.flood))],
            proposed: vec![ProposedEffect {
                slot: 0,
                magnitude: Magnitude::None,
            }],
            speech: None,
        };
        assert_eq!(
            rejected(utter(&mut kernel, &active, speech.listener, recant())),
            vec![ActionMismatch::FactUnknown { precondition: 0 }]
        );

        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::AcquireKnowledge {
                subject: Ref::Existing(speech.listener),
                fact: Ref::Existing(speech.flood),
                source: AuthoredSource::Witnessed,
                confidence: Confidence::Believed,
            }]),
        );
        let active = kernel.snapshot().unwrap();
        assert_eq!(
            rejected(utter(&mut kernel, &active, speech.listener, recant())),
            vec![ActionMismatch::FactUnknown { precondition: 0 }],
            "Believed falls short of Certain"
        );

        submit_owner(
            &mut kernel,
            &active,
            operations(vec![acquire(
                speech.listener,
                speech.flood,
                AuthoredSource::Witnessed,
            )]),
        );
        let active = kernel.snapshot().unwrap();
        utter(&mut kernel, &active, speech.listener, recant())
            .expect("certainty admits the recant");
        // Absence is the one shape of knowing nothing: no empty inner map.
        assert!(!kernel.state.knowledge.contains_key(&speech.listener));

        let active = kernel.snapshot().unwrap();
        assert_eq!(
            reject_owner(
                &mut kernel,
                &active,
                operations(vec![ComponentOp::Forget {
                    subject: Ref::Existing(speech.listener),
                    fact: Ref::Existing(speech.flood),
                }]),
            ),
            vec![Mismatch::NoOperationEffect { operation: 0 }]
        );
    }

    /// A telling never overwrites and never downgrades: the knower owns its own
    /// credence.
    #[test]
    fn a_telling_never_overwrites_a_holder() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "NoOverwrite");
        submit_owner(
            &mut kernel,
            &active,
            operations(vec![acquire(
                speech.listener,
                speech.flood,
                AuthoredSource::Witnessed,
            )]),
        );
        let before = knows(&kernel, speech.listener, speech.flood);
        let active = kernel.snapshot().unwrap();

        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::Communicate {
                speaker: Ref::Existing(speech.speaker),
                fact: Ref::Existing(speech.flood),
                to: AudienceRef::Channel(Ref::Existing(speech.horn)),
            }]),
        );
        assert_eq!(knows(&kernel, speech.listener, speech.flood), before);
        assert_eq!(
            knows(&kernel, speech.bystander, speech.flood),
            Some(Knowledge {
                confidence: Confidence::Believed,
                source: KnowledgeSource::Told {
                    by: speech.speaker,
                    via: Some(speech.horn),
                },
            })
        );
    }

    /// Ontology admission: communication reaches only subjects inside the
    /// channel's reach, and a speaker outside it is not speaking.
    #[test]
    fn communicating_from_outside_the_audience_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "OutsideAudience");
        assert_eq!(
            reject_owner(
                &mut kernel,
                &active,
                operations(vec![ComponentOp::Communicate {
                    speaker: Ref::Existing(speech.stranger),
                    fact: Ref::Existing(speech.flood),
                    to: AudienceRef::Colocated,
                }]),
            ),
            vec![Mismatch::SpeakerOutsideAudience { operation: 0 }]
        );
        assert_eq!(
            reject_owner(
                &mut kernel,
                &active,
                operations(vec![ComponentOp::Communicate {
                    speaker: Ref::Existing(speech.stranger),
                    fact: Ref::Existing(speech.flood),
                    to: AudienceRef::Channel(Ref::Existing(speech.horn)),
                }]),
            ),
            vec![Mismatch::SpeakerOutsideAudience { operation: 0 }]
        );
        assert!(kernel.state.knowledge.is_empty());
    }

    /// The two allocation sites, asserted: only `derive_id` allocates, and the
    /// action lane's one referent per invocation is reproduced by calling it
    /// directly with the recorded world, command, handle, and discriminator.
    #[test]
    fn every_canonical_id_comes_from_one_of_two_sites() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "Allocation");
        let world_id = kernel.state.world_id;
        let command_id = CommandId::new();
        let opportunity = opportunity_for(&active, speech.speaker);
        let caller = CallerId::Controller(opportunity.controller_id);
        let entities_before = kernel.state.entities.len();
        kernel
            .submit(
                command(
                    &active,
                    command_id,
                    caller.clone(),
                    CommandBody::ExerciseDecision {
                        opportunity,
                        invocation: say(
                            speech.whisper,
                            vec![binding("target", Target::Subject(speech.listener))],
                            "Counted.",
                        ),
                    },
                ),
                &AuthenticatedCaller::fixture(caller),
            )
            .expect("the whisper commits");

        let claim = minted_claim(&kernel);
        assert_eq!(
            claim,
            EntityId(patch::derive_id(
                patch::ENTITY_NAMESPACE,
                world_id,
                command_id,
                &DraftHandle::new("ghostlight.speech"),
                Some("0"),
            ))
        );
        // Exactly one referent, always a fact, always claimed by the actor.
        assert_eq!(kernel.state.entities.len(), entities_before + 1);
        assert_eq!(kernel.state.entities[&claim].kind, EntityKind::Fact);
        assert_eq!(
            kernel.state.facts[&claim].standing,
            FactStanding::Claimed { by: speech.speaker }
        );
    }

    /// Replay is exact because every allocation derives from committed inputs
    /// and every fan-out is re-derived at apply from the audience plus live
    /// state.
    #[test]
    fn restart_replay_after_speech_is_exact() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let (speech, world_id, expected) = {
            let mut kernel = WorldKernel::create(
                &path,
                creation(CommandId::new(), "ReplaySpeech"),
                &auth_principal(owner()),
            )
            .expect("a created world")
            .0;
            let (speech, active) = speech_world(&mut kernel);
            utter(
                &mut kernel,
                &active,
                speech.speaker,
                say(
                    speech.proclaim,
                    vec![binding("channel", Target::Entity(speech.horn))],
                    "One.",
                ),
            )
            .expect("the first proclamation commits");
            let active = kernel.snapshot().unwrap();
            utter(
                &mut kernel,
                &active,
                speech.listener,
                say(
                    speech.whisper,
                    vec![binding("target", Target::Subject(speech.speaker))],
                    "Two.",
                ),
            )
            .expect("the whisper commits");
            let active = kernel.snapshot().unwrap();
            submit_owner(
                &mut kernel,
                &active,
                operations(vec![ComponentOp::Communicate {
                    speaker: Ref::Existing(speech.speaker),
                    fact: Ref::Existing(speech.flood),
                    to: AudienceRef::Channel(Ref::Existing(speech.horn)),
                }]),
            );
            let world_id = kernel.state.world_id;
            (speech, world_id, kernel.state.clone())
        };

        let reopened = WorldKernel::open(&path, world_id).expect("the store replays");
        assert_eq!(reopened.state.facts, expected.facts);
        assert_eq!(reopened.state.channels, expected.channels);
        assert_eq!(reopened.state.knowledge, expected.knowledge);
        assert_eq!(reopened.state.events, expected.events);
        assert_eq!(reopened.state, expected);
        assert!(!reopened.state.knowledge[&speech.bystander].is_empty());
    }

    /// A forged claim does not apply, and a forged recipient list is
    /// unrepresentable: `ResolvedOp::Communicate` stores an audience, never
    /// recipients.
    #[test]
    fn a_forged_claim_or_forged_recipient_does_not_apply() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "Forgery");
        let command_id = CommandId::new();
        let opportunity = opportunity_for(&active, speech.speaker);
        let caller = CallerId::Controller(opportunity.controller_id);
        let invocation = say(
            speech.proclaim,
            vec![binding("channel", Target::Entity(speech.horn))],
            "Honest.",
        );
        let granted = require_granted(&kernel.state, &opportunity, invocation.affordance).unwrap();
        let honest = action::exercise(
            &kernel.state,
            command_id,
            &opportunity,
            &granted,
            &invocation,
        )
        .expect("the honest event derives");
        let fact = honest.speech.expect("the honest claim");

        let mut forged_claim = honest.clone();
        forged_claim.speech = Some(speech.flood);
        let mut forged_statement = honest.clone();
        forged_statement.effects[0] = ResolvedOp::AssertClaim {
            fact,
            statement: Statement::new("Forged.").unwrap(),
            by: speech.speaker,
        };
        let mut forged_asserter = honest.clone();
        forged_asserter.effects[0] = ResolvedOp::AssertClaim {
            fact,
            statement: Statement::new("Honest.").unwrap(),
            by: speech.listener,
        };
        for forged in [forged_claim, forged_statement, forged_asserter] {
            let mut candidate = kernel.state.clone();
            let error = apply_effect(
                &mut candidate,
                command_id,
                &caller,
                &WorldEffect::DecisionExercised {
                    opportunity: opportunity.clone(),
                    invocation: invocation.clone(),
                    event: forged,
                },
            )
            .unwrap_err();
            assert!(matches!(error, KernelError::Invariant(_)));
            assert_eq!(candidate, kernel.state);
        }

        // A recipient list cannot be forged because none is stored.
        assert!(matches!(
            honest.effects[1],
            ResolvedOp::Communicate {
                to: Audience::Channel(_),
                ..
            }
        ));
        let mut candidate = kernel.state.clone();
        apply_effect(
            &mut candidate,
            command_id,
            &caller,
            &WorldEffect::DecisionExercised {
                opportunity,
                invocation,
                event: honest,
            },
        )
        .expect("the honest event applies");
        assert!(candidate.knowledge.contains_key(&speech.listener));
    }

    /// The scope table: a telling rebinds its listener and leaves a bystander
    /// alone, and a reach change moves only the controller's digest.
    #[test]
    fn a_telling_moves_only_the_listeners_scope() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "ScopeMove");
        let digest = |kernel: &WorldKernel, subject: SubjectId| {
            scope_digest(
                &kernel.state,
                DecisionScope {
                    subject_id: subject,
                },
            )
            .unwrap()
        };
        let listener_before = digest(&kernel, speech.listener);
        let bystander_before = digest(&kernel, speech.bystander);

        utter(
            &mut kernel,
            &active,
            speech.speaker,
            say(
                speech.whisper,
                vec![binding("target", Target::Subject(speech.listener))],
                "Only you.",
            ),
        )
        .expect("the whisper commits");
        assert_ne!(digest(&kernel, speech.listener), listener_before);
        assert_eq!(digest(&kernel, speech.bystander), bystander_before);

        // Reach membership is admission-only: adding a hearer moves the
        // controller's digest and nobody else's.
        let active = kernel.snapshot().unwrap();
        let hearer_before = digest(&kernel, speech.listener);
        let speaker_before = digest(&kernel, speech.speaker);
        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::SetReach {
                channel: Ref::Existing(speech.horn),
                reach: ReachRef::Subjects(BTreeSet::from([Ref::Existing(speech.stranger)])),
            }]),
        );
        assert_eq!(digest(&kernel, speech.listener), hearer_before);
        assert_ne!(digest(&kernel, speech.speaker), speaker_before);
    }

    /// A speech-carrying entry names exactly one audience: none is unlowerable
    /// and two is unchoosable. Each rejection allocates no `AffordanceId`.
    #[test]
    fn a_speech_carrying_entry_must_declare_exactly_one_audience() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = WorldKernel::create(
            directory.path().join("world.cc"),
            creation(CommandId::new(), "SpeechAudience"),
            &auth_principal(owner()),
        )
        .expect("a draft world")
        .0;
        let before = kernel.snapshot().unwrap();
        let catalog_before = kernel.state.affordance_catalog.len();
        let declare = |preconditions: Vec<Precondition>| CommandBody::AdmitPatch {
            answers: None,
            patch: WorldPatch {
                declarations: vec![Declaration::Affordance(AffordanceDeclaration {
                    handle: DraftHandle::new("herald"),
                    kind: AffordanceKindName("herald".into()),
                    roles: Vec::new(),
                    preconditions,
                    effect_slots: Vec::new(),
                    outcome_bands: vec![OutcomeBand {
                        weight: 1,
                        effects: Vec::new(),
                    }],
                    carries_speech: true,
                })],
                operations: Vec::new(),
                evidence: Vec::new(),
            },
        };
        assert_eq!(
            reject_owner(&mut kernel, &before, declare(Vec::new())),
            vec![Mismatch::SpeechWithoutAudience {
                handle: DraftHandle::new("herald"),
            }]
        );
        assert_eq!(
            reject_owner(
                &mut kernel,
                &before,
                declare(vec![
                    Precondition::CanBroadcast {
                        via: AudienceSpec::Colocated,
                    },
                    Precondition::CanBroadcast {
                        via: AudienceSpec::Colocated,
                    },
                ]),
            ),
            vec![Mismatch::AmbiguousSpeechAudience {
                handle: DraftHandle::new("herald"),
            }]
        );
        assert_eq!(kernel.state.affordance_catalog.len(), catalog_before);
    }

    /// The kernel does not compare two statements: no dedup, no interning, no
    /// contradiction check, no similarity. A claim byte-identical to a
    /// canonical fact and one that plainly negates it commit identically.
    #[test]
    fn the_kernel_never_compares_two_statements() {
        let directory = tempfile::tempdir().unwrap();
        let run = |name: &str, text: &str| {
            let room = directory.path().join(name);
            std::fs::create_dir_all(&room).unwrap();
            let (mut kernel, speech, active) = speech_kernel(&room, name);
            let receipt = utter(
                &mut kernel,
                &active,
                speech.speaker,
                say(
                    speech.whisper,
                    vec![binding("target", Target::Subject(speech.listener))],
                    text,
                ),
            )
            .expect("both statements commit identically");
            assert!(matches!(receipt, SubmitReceipt::Applied(_)));
            let event = kernel.state.events.last().unwrap().clone();
            (kernel, speech, event)
        };
        let (agreeing, agreeing_speech, agreeing_event) = run("agree", FLOOD_STATEMENT);
        let (negating, _, negating_event) = run("negate", "The lower hinge is dry.");

        assert_eq!(agreeing_event.band, negating_event.band);
        assert_eq!(agreeing_event.effects.len(), negating_event.effects.len());
        assert_eq!(agreeing.state.facts.len(), negating.state.facts.len());
        // Two facts with byte-identical statements are two facts: the claim is
        // not the canonical fact it repeats, and neither run learned anything
        // about the other's meaning.
        let claim = agreeing_event.speech.unwrap();
        assert_ne!(claim, agreeing_speech.flood);
        assert_eq!(
            agreeing.state.facts[&claim].statement,
            agreeing.state.facts[&agreeing_speech.flood].statement
        );
        assert_ne!(
            agreeing.state.facts[&claim].standing,
            agreeing.state.facts[&agreeing_speech.flood].standing
        );
    }
}

/// Soul's pass-6 falsification set. Each test names the claim it attacks and,
/// where the landed behaviour differs from the specified one, pins the actual
/// behaviour so the difference is visible rather than assumed.
#[cfg(test)]
mod soul_knowledge_tests {
    use super::tests::{
        FLOOD_STATEMENT, Speech, affordance_named, auth_principal, command, creation, operations,
        opportunity_for, owner, speech_world, submit_owner,
    };
    use super::*;
    use std::path::Path;

    fn speech_kernel(path: &Path, title: &str) -> (WorldKernel, Speech, WorldSnapshot) {
        let mut kernel = WorldKernel::create(
            path.join("world.cc"),
            creation(CommandId::new(), title),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let (speech, active) = speech_world(&mut kernel);
        (kernel, speech, active)
    }

    fn say(affordance: AffordanceId, bindings: Vec<RoleBinding>, text: &str) -> DecisionInvocation {
        DecisionInvocation {
            affordance,
            bindings,
            proposed: Vec::new(),
            speech: Some(Statement::new(text).unwrap()),
        }
    }

    fn binding(role: &str, target: Target) -> RoleBinding {
        RoleBinding {
            role: Role(role.into()),
            target,
        }
    }

    fn utter(
        kernel: &mut WorldKernel,
        snapshot: &WorldSnapshot,
        actor: SubjectId,
        invocation: DecisionInvocation,
    ) -> Result<SubmitReceipt, KernelError> {
        let opportunity = opportunity_for(snapshot, actor);
        let caller = CallerId::Controller(opportunity.controller_id);
        kernel.submit(
            command(
                snapshot,
                CommandId::new(),
                caller.clone(),
                CommandBody::ExerciseDecision {
                    opportunity,
                    invocation,
                },
            ),
            &AuthenticatedCaller::fixture(caller),
        )
    }

    fn knows(kernel: &WorldKernel, subject: SubjectId, fact: EntityId) -> Option<Knowledge> {
        kernel
            .state
            .knowledge
            .get(&subject)
            .and_then(|held| held.get(&fact))
            .copied()
    }

    fn digest_of(kernel: &WorldKernel, subject_id: SubjectId) -> ScopeDigest {
        scope_digest(&kernel.state, DecisionScope { subject_id }).unwrap()
    }

    fn minted_claim(kernel: &WorldKernel) -> EntityId {
        kernel
            .state
            .events
            .last()
            .expect("the committed event")
            .speech
            .expect("a speech act names its claim")
    }

    /// A channel's controller carries exactly one privilege: `can_broadcast`
    /// lets it sound the horn from outside the declared reach. `audience`
    /// itself never folds the controller in, so the controller is not a
    /// recipient of a telling it did not stand inside, whether the telling is
    /// its own or another subject's.
    #[test]
    fn soul_a_channel_controller_hears_what_it_is_out_of_reach_of() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "ControllerHears");
        // The horn now reaches exactly one subject. The speaker controls it and
        // is not in that set.
        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::SetReach {
                channel: Ref::Existing(speech.horn),
                reach: ReachRef::Subjects(BTreeSet::from([Ref::Existing(speech.listener)])),
            }]),
        );
        let active = kernel.snapshot().unwrap();
        assert_eq!(
            kernel.state.channels[&speech.horn].reach,
            Reach::Subjects(BTreeSet::from([speech.listener]))
        );

        // The controller may still speak from outside its own reach — the horn
        // belongs to the temple — and the listener, who stands inside the
        // reach, hears it.
        utter(
            &mut kernel,
            &active,
            speech.speaker,
            say(
                speech.proclaim,
                vec![binding("channel", Target::Entity(speech.horn))],
                "The horn is mine to sound.",
            ),
        )
        .expect("the controller may speak on its own channel");
        let controllers_claim = minted_claim(&kernel);
        assert_eq!(
            knows(&kernel, speech.listener, controllers_claim),
            Some(Knowledge {
                confidence: Confidence::Believed,
                source: KnowledgeSource::Told {
                    by: speech.speaker,
                    via: Some(speech.horn),
                },
            })
        );
        // The controller gains no knowledge of its own claim: it is outside
        // the reach, and speaking privilege is not audience membership.
        assert_eq!(knows(&kernel, speech.speaker, controllers_claim), None);

        // Now the listener speaks on the same channel.
        let active = kernel.snapshot().unwrap();
        utter(
            &mut kernel,
            &active,
            speech.listener,
            say(
                speech.proclaim,
                vec![binding("channel", Target::Entity(speech.horn))],
                "The horn is mine tonight.",
            ),
        )
        .expect("a subject inside the reach may broadcast");
        let claim = minted_claim(&kernel);

        // The declared reach names the listener alone, and the listener spoke,
        // so the declared fan-out is empty. The controller — outside the
        // reach, and not the speaker this time either — learns nothing.
        assert_eq!(
            knows(&kernel, speech.speaker, claim),
            None,
            "the controller does not gain knowledge through fan-out"
        );
        assert_eq!(knows(&kernel, speech.bystander, claim), None);
        assert_eq!(knows(&kernel, speech.stranger, claim), None);
        assert_eq!(knows(&kernel, speech.listener, claim), None);
    }

    /// Every speech precondition fails at `exercise`, before the band draw and
    /// before the speech lowering, so a refused utterance mints no claim and
    /// leaves the whole committed state byte-identical. Nothing here reaches
    /// replay to be caught later.
    #[test]
    fn soul_a_refused_utterance_dies_at_the_gate_and_allocates_nothing() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "GateNotReplay");
        let speak = affordance_named(&active, "speak");
        let before = kernel.state.clone();

        let cases: Vec<(SubjectId, DecisionInvocation, Vec<ActionMismatch>)> = vec![
            // Unplaced: no co-location audience at all.
            (
                speech.stranger,
                say(speak, Vec::new(), "Anyone?"),
                vec![ActionMismatch::NoAudience { precondition: 0 }],
            ),
            // Addressed at a subject one containment level down.
            (
                speech.speaker,
                say(
                    speech.whisper,
                    vec![binding("target", Target::Subject(speech.bystander))],
                    "Down there.",
                ),
                vec![ActionMismatch::CannotReach { precondition: 0 }],
            ),
            // `recant` requires `Knows { at_least: Certain }` of the bound fact.
            // Its unproposed slot names itself beside the precondition; the
            // point is that the complete set lands at the gate, in one pass.
            (
                speech.speaker,
                DecisionInvocation {
                    affordance: speech.recant,
                    bindings: vec![binding("fact", Target::Entity(speech.flood))],
                    proposed: Vec::new(),
                    speech: None,
                },
                vec![
                    ActionMismatch::SlotNotProposed { slot: 0 },
                    ActionMismatch::FactUnknown { precondition: 0 },
                ],
            ),
        ];

        for (actor, invocation, expected) in cases {
            let error = utter(&mut kernel, &active, actor, invocation).unwrap_err();
            let KernelError::ActionRejected(mismatches) = error else {
                panic!("expected an action rejection, got {error:?}");
            };
            assert_eq!(mismatches, expected);
            // No claim, no entity row, no event, no revision: the gate ran
            // before anything was drawn or lowered.
            assert_eq!(kernel.state, before);
        }
    }

    /// Fan-out is the sole way knowledge enters another subject from speech, and
    /// it is not carried by the effect. A forged `DecisionExercised` that widens
    /// the audience, appends a second telling, or tells a foreign fact is refused
    /// by `apply_effect` itself, so replay is never the first reader to notice.
    #[test]
    fn soul_a_forged_wider_audience_does_not_apply() {
        let directory = tempfile::tempdir().unwrap();
        let (kernel, speech, active) = speech_kernel(directory.path(), "WiderAudience");
        let command_id = CommandId::new();
        let opportunity = opportunity_for(&active, speech.speaker);
        let caller = CallerId::Controller(opportunity.controller_id);
        let invocation = say(
            speech.whisper,
            vec![binding("target", Target::Subject(speech.listener))],
            "Only the hall.",
        );
        let granted = require_granted(&kernel.state, &opportunity, invocation.affordance).unwrap();
        let honest = action::exercise(
            &kernel.state,
            command_id,
            &opportunity,
            &granted,
            &invocation,
        )
        .expect("the honest event derives");
        let fact = honest.speech.expect("the honest claim");
        assert_eq!(
            honest.effects[1],
            ResolvedOp::Communicate {
                speaker: speech.speaker,
                fact,
                to: Audience::Colocated,
            }
        );

        // Widened to the horn, which covers the yard through `covers_place`.
        let mut widened = honest.clone();
        widened.effects[1] = ResolvedOp::Communicate {
            speaker: speech.speaker,
            fact,
            to: Audience::Channel(speech.horn),
        };
        // A second telling appended to the same event.
        let mut doubled = honest.clone();
        doubled.effects.push(ResolvedOp::Communicate {
            speaker: speech.speaker,
            fact: speech.flood,
            to: Audience::Colocated,
        });
        // A telling of a fact this act did not mint.
        let mut foreign = honest.clone();
        foreign.effects[1] = ResolvedOp::Communicate {
            speaker: speech.speaker,
            fact: speech.flood,
            to: Audience::Colocated,
        };

        for forged in [widened, doubled, foreign] {
            let mut candidate = kernel.state.clone();
            let error = apply_effect(
                &mut candidate,
                command_id,
                &caller,
                &WorldEffect::DecisionExercised {
                    opportunity: opportunity.clone(),
                    invocation: invocation.clone(),
                    event: forged,
                },
            )
            .unwrap_err();
            assert!(matches!(error, KernelError::Invariant(_)));
            assert_eq!(candidate, kernel.state);
        }
    }

    /// A speech act that lands nothing moves no digest at all, the speaker's own
    /// included. `AssertClaim` writes `entities` and `facts`, and neither is a
    /// scope component, so an in-flight proposal held by anyone in the world
    /// survives an utterance nobody heard.
    #[test]
    fn soul_speech_that_lands_nothing_moves_no_digest_including_the_speakers() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "NoDigestMove");
        let speak = affordance_named(&active, "speak");
        let everyone = [
            speech.speaker,
            speech.listener,
            speech.bystander,
            speech.stranger,
        ];
        let before: Vec<ScopeDigest> = everyone
            .iter()
            .map(|subject| digest_of(&kernel, *subject))
            .collect();
        let facts_before = kernel.state.facts.len();

        // The bystander stands alone in the yard.
        utter(
            &mut kernel,
            &active,
            speech.bystander,
            say(speak, Vec::new(), "Nobody is here."),
        )
        .expect("speaking into an empty room commits");

        assert_eq!(kernel.state.facts.len(), facts_before + 1);
        assert_eq!(kernel.state.events.len(), 1);
        assert!(kernel.state.knowledge.is_empty());
        for (subject, digest) in everyone.iter().zip(before) {
            assert_eq!(
                digest_of(&kernel, *subject),
                digest,
                "an utterance nobody heard moved a scope digest"
            );
        }
    }

    /// The resolver does not model a `Communicate`'s fan-out, so a same-patch
    /// telling followed by a `Forget` of what that telling would have landed is
    /// refused at resolve although apply would accept it. The asymmetry is
    /// fail-closed, and it does not refuse the order that must work: forgetting
    /// first and being told again in the same patch commits, and the telling is
    /// what the subject ends up holding.
    #[test]
    fn soul_a_same_patch_telling_then_forget_fails_closed_only() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "SamePatchOrder");
        let tell = ComponentOp::Communicate {
            speaker: Ref::Existing(speech.speaker),
            fact: Ref::Existing(speech.flood),
            to: AudienceRef::Colocated,
        };
        let forget = ComponentOp::Forget {
            subject: Ref::Existing(speech.listener),
            fact: Ref::Existing(speech.flood),
        };

        let error = kernel
            .submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    operations(vec![tell.clone(), forget.clone()]),
                ),
                &auth_principal(owner()),
            )
            .unwrap_err();
        let KernelError::PatchRejected(mismatches) = error else {
            panic!("expected a rejected patch, got {error:?}");
        };
        assert_eq!(
            mismatches,
            vec![Mismatch::NoOperationEffect { operation: 1 }]
        );

        // The legitimate order. The listener holds the fact first, forgets it,
        // and the telling in the same patch lands it again as a telling, which
        // is what the committed state carries.
        submit_owner(
            &mut kernel,
            &active,
            operations(vec![ComponentOp::AcquireKnowledge {
                subject: Ref::Existing(speech.listener),
                fact: Ref::Existing(speech.flood),
                source: AuthoredSource::Witnessed,
                confidence: Confidence::Certain,
            }]),
        );
        let active = kernel.snapshot().unwrap();
        submit_owner(&mut kernel, &active, operations(vec![forget, tell]));
        assert_eq!(
            knows(&kernel, speech.listener, speech.flood),
            Some(Knowledge {
                confidence: Confidence::Believed,
                source: KnowledgeSource::Told {
                    by: speech.speaker,
                    via: None,
                },
            })
        );
    }

    /// `verify_state_shape` checks the forward speech direction only: an event's
    /// claim is asserted by the acting subject and named by no other event. The
    /// reverse, that every claim is named by some event, is deliberately absent,
    /// because a world may declare a claim in Draft with no event behind it.
    /// That gap is not a hole: active admission refuses every declaration, so
    /// `AssertClaim` is the only way a claim enters an active world and it is
    /// synthesized with its event.
    #[test]
    fn soul_an_orphan_claim_is_a_draft_shape_and_unreachable_in_active() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "OrphanClaim");
        // The Draft-declared fact is an orphan by construction: no event names
        // it. Reopening runs the shape verifier over the replayed state.
        assert!(kernel.state.facts.contains_key(&speech.flood));
        assert!(
            kernel
                .state
                .events
                .iter()
                .all(|event| event.speech.is_none())
        );
        let world_id = kernel.state.world_id;

        let before = kernel.state.clone();
        let error = kernel
            .submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    CommandBody::AdmitPatch {
                        answers: None,
                        patch: WorldPatch {
                            declarations: vec![Declaration::Fact(FactDeclaration {
                                handle: DraftHandle::new("rumour"),
                                label: "a rumour".into(),
                                statement: Statement::new("The gate is unguarded.").unwrap(),
                                standing: FactStandingRef::Claimed {
                                    by: Ref::Existing(speech.speaker),
                                },
                            })],
                            operations: Vec::new(),
                            evidence: Vec::new(),
                        },
                    },
                ),
                &auth_principal(owner()),
            )
            .unwrap_err();
        assert!(
            matches!(error, KernelError::AnswerRequired),
            "an active world admitted an unanswered declaration: {error:?}"
        );
        assert_eq!(kernel.state, before);
        drop(kernel);
        let replayed = WorldKernel::open(&path, world_id).expect("the orphan store is healthy");
        assert_eq!(replayed.state.facts, before.facts);
    }

    /// Replay equality across every new arm, twice, with no RNG and no clock in
    /// any of them: a speech act, a telling, an authored acquisition, a forget,
    /// a reach change, and a controller change all reproduce byte-identically
    /// from the journal, and reopening again reproduces the same state.
    #[test]
    fn soul_replay_is_exact_across_every_new_arm() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let (world_id, expected) = {
            let (mut kernel, speech, active) = speech_kernel(directory.path(), "ReplayArms");
            utter(
                &mut kernel,
                &active,
                speech.speaker,
                say(
                    speech.proclaim,
                    vec![binding("channel", Target::Entity(speech.horn))],
                    "The hinge is flooding.",
                ),
            )
            .expect("the proclamation commits");
            let active = kernel.snapshot().unwrap();
            submit_owner(
                &mut kernel,
                &active,
                operations(vec![
                    ComponentOp::AcquireKnowledge {
                        subject: Ref::Existing(speech.stranger),
                        fact: Ref::Existing(speech.flood),
                        source: AuthoredSource::Evidenced,
                        confidence: Confidence::Certain,
                    },
                    ComponentOp::Communicate {
                        speaker: Ref::Existing(speech.speaker),
                        fact: Ref::Existing(speech.flood),
                        to: AudienceRef::Colocated,
                    },
                    ComponentOp::SetController {
                        channel: Ref::Existing(speech.horn),
                        controller: Some(Ref::Existing(speech.listener)),
                    },
                    ComponentOp::SetReach {
                        channel: Ref::Existing(speech.horn),
                        reach: ReachRef::Place(Ref::Existing(speech.yard)),
                    },
                ]),
            );
            let active = kernel.snapshot().unwrap();
            submit_owner(
                &mut kernel,
                &active,
                operations(vec![ComponentOp::Forget {
                    subject: Ref::Existing(speech.stranger),
                    fact: Ref::Existing(speech.flood),
                }]),
            );
            assert!(!kernel.state.knowledge.is_empty());
            assert!(!kernel.state.facts.is_empty());
            (kernel.state.world_id, kernel.state.clone())
        };

        for _ in 0..2 {
            let replayed = WorldKernel::open(&path, world_id).expect("the store replays");
            assert_eq!(replayed.state.facts, expected.facts);
            assert_eq!(replayed.state.channels, expected.channels);
            assert_eq!(replayed.state.knowledge, expected.knowledge);
            assert_eq!(replayed.state.entities, expected.entities);
            assert_eq!(replayed.state.events, expected.events);
            assert_eq!(replayed.state, expected);
        }
    }

    /// The statement's bytes are stored twice, and only one of the two copies
    /// is a read surface. `facts[fact].statement` is the readable home:
    /// `spoken()` and every projection consult it. The committed event's own
    /// `AssertClaim` effect carries the same bytes as the replay witness —
    /// required so `apply_effect` can re-derive the identical event on replay
    /// without re-deriving the statement from elsewhere — but nothing reads
    /// that copy back out. Those two are the whole of it, and neither reaches
    /// a subject-facing surface.
    #[test]
    fn soul_the_utterance_is_stored_in_facts_and_in_the_committed_effect() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "OneHome");
        let speak = affordance_named(&active, "speak");
        let text = "A statement that appears nowhere else.";
        utter(
            &mut kernel,
            &active,
            speech.speaker,
            say(speak, Vec::new(), text),
        )
        .expect("the kernel speak commits");
        let claim = minted_claim(&kernel);

        assert_eq!(kernel.state.facts[&claim].statement.as_str(), text);
        assert_eq!(
            kernel.state.entities[&claim].label, CLAIM_LABEL,
            "the label is a name, not a transcript"
        );
        // The event no longer embeds the invocation, and its `speech` field is
        // the fact id. The bytes still appear in the event, inside the committed
        // `AssertClaim` effect.
        let event = kernel.state.events.last().unwrap();
        assert_eq!(event.speech, Some(claim));
        assert_eq!(
            event.effects[0],
            ResolvedOp::AssertClaim {
                fact: claim,
                statement: Statement::new(text).unwrap(),
                by: speech.speaker,
            },
            "the event carries the statement as a replay witness, not a second read surface"
        );
        // Those two are the whole of it: strip `facts` and `events` and the
        // rest of the state is silent.
        let mut stripped = kernel.state.clone();
        stripped.facts.clear();
        stripped.events.clear();
        let encoded = rmp_serde::to_vec_named(&stripped).unwrap();
        assert!(
            !encoded
                .windows(text.len())
                .any(|window| window == text.as_bytes())
        );
        // The statement declared in Draft is untouched by any of it.
        assert_eq!(
            kernel.state.facts[&speech.flood].statement.as_str(),
            FLOOD_STATEMENT
        );
    }

    /// `operator_log` is unscoped by construction: it renders every committed
    /// act's utterance regardless of who holds knowledge of it. That is correct
    /// for the human operator's story feed and is exactly why it must never
    /// become reachable from a controller lane. `ControllerRunner` no longer
    /// owns a `WorldMailbox`; it owns a `ControllerPort`, which forwards
    /// exactly the five requests a controller lane makes and has no
    /// `operator_log` method at all. The separation is a type boundary now:
    /// `WorldMailbox::operator_log` stays `pub(crate)` for the operator/story
    /// feed owner, but nothing inside `controllers.rs` can name it.
    #[test]
    fn soul_the_operator_log_is_unscoped_and_must_stay_owner_only() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, speech, active) = speech_kernel(directory.path(), "OperatorLog");
        let text = "Nobody in this room but me.";
        utter(
            &mut kernel,
            &active,
            speech.bystander,
            say(affordance_named(&active, "speak"), Vec::new(), text),
        )
        .expect("speaking alone in the yard commits");

        // Nobody holds it.
        assert!(kernel.state.knowledge.is_empty());
        // The operator reads it anyway, with the speaker resolved by label.
        let log = operator_log(&kernel.state).unwrap();
        assert_eq!(
            log,
            vec![OperatorEvent {
                revision: kernel.state.revision,
                speaker: speech.bystander,
                speaker_label: "The Yard Bystander".into(),
                speech: Some(Statement::new(text).unwrap()),
            }]
        );
        // And no subject snapshot carries it.
        for subject in kernel.snapshot().unwrap().subjects {
            assert!(subject.knowledge.is_empty());
        }
    }
}

/// The clock, commitments, pressure, ordered attention, boundaries, and scale.
#[cfg(test)]
mod clock_tests {
    use super::patch::{PreconditionRef, PressureSourceRef, WorldScaleIntentRef};
    use super::tests::{
        activate, affordance_named, auth_principal, command, creation, operations, opportunity_for,
        owner, player, reject_owner, submit_owner,
    };
    use super::*;
    use std::path::Path;

    /// The canonical IDs of the clock fixture: a hall containing a yard, a
    /// dead end reached by exactly one route, three subjects, a resource, and
    /// one commitment of each kind.
    pub(super) struct Clockwork {
        yard: EntityId,
        dead_end: EntityId,
        gate: EdgeId,
        reeve: SubjectId,
        farmer: SubjectId,
        treasury: SubjectId,
        routine: CommitmentKey,
        obligation: CommitmentKey,
        goal: CommitmentKey,
        deliver: AffordanceId,
        threaten: AffordanceId,
    }

    const ROUTINE_DUE: u64 = 60;
    const LATE_DUE: u64 = 100;

    fn minutes(value: u32) -> TickMinutes {
        TickMinutes::new(value).expect("a valid span")
    }

    fn clock_kernel(path: &Path, title: &str) -> (WorldKernel, Clockwork, WorldSnapshot) {
        let mut kernel = WorldKernel::create(
            path.join("world.cc"),
            creation(CommandId::new(), title),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let (clockwork, active) = clock_world(&mut kernel);
        (kernel, clockwork, active)
    }

    fn clock_world(kernel: &mut WorldKernel) -> (Clockwork, WorldSnapshot) {
        let before = kernel.snapshot().unwrap();
        let speak = super::tests::speak_entry(kernel);
        let person = |handle: &str, label: &str, kind: SubjectKind, place: &str| {
            Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new(handle),
                label: label.into(),
                kind,
                controller: NewController::NarrativePersona,
                affordances: BTreeSet::from([
                    speak.clone(),
                    Ref::Draft(DraftHandle::new("deliver")),
                    Ref::Draft(DraftHandle::new("threaten")),
                ]),
                position: Some(Ref::Draft(DraftHandle::new(place))),
            })
        };
        let subject = |handle: &str| Ref::Draft(DraftHandle::new(handle));
        submit_owner(
            kernel,
            &before,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    evidence: vec![EvidenceRef::new("vault:harvest")],
                    declarations: vec![
                        Declaration::Entity(EntityDeclaration {
                            handle: DraftHandle::new("hall"),
                            label: "The Long Hall".into(),
                            kind: EntityKind::Place,
                            container: None,
                        }),
                        Declaration::Entity(EntityDeclaration {
                            handle: DraftHandle::new("yard"),
                            label: "The Cavity Yard".into(),
                            kind: EntityKind::Place,
                            container: Some(Ref::Draft(DraftHandle::new("hall"))),
                        }),
                        // Nothing inside it, nobody in it, and one route out:
                        // the world has not grown into it yet.
                        Declaration::Entity(EntityDeclaration {
                            handle: DraftHandle::new("dead-end"),
                            label: "The Unwalked Road".into(),
                            kind: EntityKind::Place,
                            container: None,
                        }),
                        Declaration::Entity(EntityDeclaration {
                            handle: DraftHandle::new("grain"),
                            label: "Winter Grain".into(),
                            kind: EntityKind::Resource,
                            container: None,
                        }),
                        Declaration::Route(RouteDeclaration {
                            handle: DraftHandle::new("stair"),
                            label: "The Yard Stair".into(),
                            from: Ref::Draft(DraftHandle::new("yard")),
                            to: Ref::Draft(DraftHandle::new("hall")),
                            access: AccessKind::Public,
                            cost: Cost(1),
                        }),
                        Declaration::Route(RouteDeclaration {
                            handle: DraftHandle::new("back"),
                            label: "The Hall Steps".into(),
                            from: Ref::Draft(DraftHandle::new("hall")),
                            to: Ref::Draft(DraftHandle::new("yard")),
                            access: AccessKind::Public,
                            cost: Cost(1),
                        }),
                        Declaration::Route(RouteDeclaration {
                            handle: DraftHandle::new("gate"),
                            label: "The Field Gate".into(),
                            from: Ref::Draft(DraftHandle::new("hall")),
                            to: Ref::Draft(DraftHandle::new("dead-end")),
                            access: AccessKind::Public,
                            cost: Cost(1),
                        }),
                        // An affordance whose legitimacy is a promise rather
                        // than a jurisdiction, and one that presses directly.
                        Declaration::Affordance(AffordanceDeclaration {
                            handle: DraftHandle::new("deliver"),
                            kind: AffordanceKindName("deliver".into()),
                            roles: vec![RoleSpec {
                                role: Role("creditor".into()),
                                kind: RefKind::Subject(None),
                            }],
                            preconditions: vec![Precondition::Committed {
                                to: Role("creditor".into()),
                                kind: CommitmentKind::Obligation,
                            }],
                            effect_slots: vec![EffectSlot {
                                op_kind: ComponentOpKind::CreateCommitment {
                                    kind: CommitmentKind::Obligation,
                                    horizon: minutes(1440),
                                    period: None,
                                },
                                roles: vec![Role("actor".into()), Role("creditor".into())],
                                bounds: Bounds::None,
                            }],
                            outcome_bands: vec![OutcomeBand {
                                weight: 1,
                                effects: vec![0],
                            }],
                            carries_speech: false,
                        }),
                        Declaration::Affordance(AffordanceDeclaration {
                            handle: DraftHandle::new("threaten"),
                            kind: AffordanceKindName("threaten".into()),
                            roles: vec![RoleSpec {
                                role: Role("target".into()),
                                kind: RefKind::Subject(None),
                            }],
                            preconditions: Vec::new(),
                            effect_slots: vec![EffectSlot {
                                op_kind: ComponentOpKind::AdvancePressure {
                                    by: PressureMagnitude(3),
                                },
                                roles: vec![Role("target".into())],
                                bounds: Bounds::None,
                            }],
                            outcome_bands: vec![OutcomeBand {
                                weight: 1,
                                effects: vec![0],
                            }],
                            carries_speech: false,
                        }),
                        person("reeve", "The Yard Reeve", SubjectKind::Person, "yard"),
                        person("farmer", "The Yard Farmer", SubjectKind::Person, "yard"),
                        person(
                            "treasury",
                            "The Hall Treasury",
                            SubjectKind::Institution,
                            "hall",
                        ),
                    ],
                    operations: vec![
                        ComponentOp::Admit {
                            holder: subject("farmer"),
                            resource: Ref::Draft(DraftHandle::new("grain")),
                            qty: Quantity(4),
                            evidence: EvidenceRef::new("vault:harvest"),
                        },
                        // 0: the routine, whose check is where its subject stands.
                        ComponentOp::CreateCommitment {
                            subject: subject("farmer"),
                            counterparty: None,
                            kind: CommitmentKind::Routine,
                            due: FictionalMinutes(ROUTINE_DUE),
                            period: Some(minutes(10)),
                            checks: vec![PreconditionRef::Present {
                                at: Ref::Draft(DraftHandle::new("yard")),
                            }],
                        },
                        ComponentOp::CreateCommitment {
                            subject: subject("farmer"),
                            counterparty: Some(subject("treasury")),
                            kind: CommitmentKind::Obligation,
                            due: FictionalMinutes(LATE_DUE),
                            period: None,
                            checks: Vec::new(),
                        },
                        ComponentOp::CreateCommitment {
                            subject: subject("reeve"),
                            counterparty: None,
                            kind: CommitmentKind::Goal,
                            due: FictionalMinutes(LATE_DUE),
                            period: None,
                            checks: Vec::new(),
                        },
                        ComponentOp::Bind {
                            subject: subject("reeve"),
                            target: DependencyRef::Route(Ref::Draft(DraftHandle::new("gate"))),
                        },
                    ],
                },
            },
        );
        let active = activate(kernel);
        let place = |label: &str| {
            active
                .places
                .iter()
                .find(|place| place.label == label)
                .expect("the declared place")
                .id
        };
        let who = |label: &str| {
            active
                .subjects
                .iter()
                .find(|subject| subject.label == label)
                .expect("the declared subject")
                .id
        };
        let reeve = who("The Yard Reeve");
        let farmer = who("The Yard Farmer");
        let key_of = |subject: SubjectId, kind: CommitmentKind| {
            *kernel
                .state
                .commitments
                .get(&subject)
                .expect("the subject holds commitments")
                .iter()
                .find(|(_, commitment)| commitment.kind == kind)
                .expect("the declared commitment")
                .0
        };
        let gate = active
            .routes
            .iter()
            .find(|route| route.label == "The Field Gate")
            .expect("the declared route")
            .id;
        (
            Clockwork {
                yard: place("The Cavity Yard"),
                dead_end: place("The Unwalked Road"),
                gate,
                reeve,
                farmer,
                treasury: who("The Hall Treasury"),
                routine: key_of(farmer, CommitmentKind::Routine),
                obligation: key_of(farmer, CommitmentKind::Obligation),
                goal: key_of(reeve, CommitmentKind::Goal),
                deliver: affordance_named(&active, "deliver"),
                threaten: affordance_named(&active, "threaten"),
            },
            active,
        )
    }

    fn clock_caller() -> CallerId {
        CallerId::System(SystemCapability::Clock)
    }

    fn tick(kernel: &mut WorldKernel, span: u32) -> Result<SubmitReceipt, KernelError> {
        let snapshot = kernel.snapshot().unwrap();
        kernel.submit(
            command(
                &snapshot,
                CommandId::new(),
                clock_caller(),
                CommandBody::AdvanceTime {
                    minutes: minutes(span),
                },
            ),
            &AuthenticatedCaller::fixture(clock_caller()),
        )
    }

    fn exercise(
        kernel: &mut WorldKernel,
        actor: SubjectId,
        affordance: AffordanceId,
        bindings: Vec<RoleBinding>,
    ) -> Result<SubmitReceipt, KernelError> {
        let snapshot = kernel.snapshot().unwrap();
        let opportunity = opportunity_for(&snapshot, actor);
        let caller = CallerId::Controller(opportunity.controller_id);
        kernel.submit(
            command(
                &snapshot,
                CommandId::new(),
                caller.clone(),
                CommandBody::ExerciseDecision {
                    opportunity,
                    invocation: DecisionInvocation {
                        affordance,
                        bindings,
                        proposed: vec![ProposedEffect {
                            slot: 0,
                            magnitude: Magnitude::None,
                        }],
                        speech: None,
                    },
                },
            ),
            &AuthenticatedCaller::fixture(caller),
        )
    }

    fn binding(role: &str, target: Target) -> RoleBinding {
        RoleBinding {
            role: Role(role.into()),
            target,
        }
    }

    fn pressure(kernel: &WorldKernel, target: SubjectId, source: PressureSource) -> u32 {
        kernel
            .state
            .pressures
            .get(&target)
            .and_then(|held| held.get(&source))
            .map_or(0, |magnitude| magnitude.0)
    }

    fn due(kernel: &WorldKernel, subject: SubjectId, key: CommitmentKey) -> FictionalMinutes {
        kernel
            .state
            .commitments
            .get(&subject)
            .and_then(|held| held.get(&key))
            .expect("the live commitment")
            .due
    }

    /// Verification 12, both halves: a defaulted commitment advances pressure on
    /// its subject, and that subject still derives exactly one opportunity.
    #[test]
    fn a_defaulted_commitment_advances_pressure_and_the_subject_derives_an_opportunity() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Default");
        let sourced = PressureSource::Commitment {
            subject: clockwork.farmer,
            key: clockwork.obligation,
        };

        tick(&mut kernel, 120).expect("the tick commits");
        assert_eq!(pressure(&kernel, clockwork.farmer, sourced), 1);
        assert_eq!(
            kernel.state.pressures[&clockwork.farmer].len(),
            1,
            "only the past-due obligation pressed the farmer"
        );
        let snapshot = kernel.snapshot().unwrap();
        assert_eq!(
            snapshot
                .opportunities
                .iter()
                .filter(|opportunity| opportunity.scope.subject_id == clockwork.farmer)
                .count(),
            1
        );

        tick(&mut kernel, 1).expect("the second tick commits");
        assert_eq!(pressure(&kernel, clockwork.farmer, sourced), 2);
    }

    /// Verification 19: a due routine fulfils with no inference and no pressure,
    /// and a blocked one neither rolls nor presses.
    #[test]
    fn a_due_routine_auto_fulfils_and_a_blocked_one_does_not() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, active) = clock_kernel(directory.path(), "Routine");
        let holdings = kernel.state.holdings.clone();
        let positions = kernel.state.positions.clone();
        let edges = kernel.state.edges.clone();

        tick(&mut kernel, ROUTINE_DUE as u32).expect("the tick commits");
        assert_eq!(
            due(&kernel, clockwork.farmer, clockwork.routine),
            FictionalMinutes(ROUTINE_DUE + 10)
        );
        assert!(
            kernel.state.pressures.is_empty(),
            "a routine raises nothing"
        );
        assert_eq!(kernel.state.holdings, holdings);
        assert_eq!(kernel.state.positions, positions);
        assert_eq!(kernel.state.edges, edges);

        // Out of the yard: the check fails, so the routine neither rolls nor
        // presses, and it retries on the next tick.
        let route = |label: &str| {
            active
                .routes
                .iter()
                .find(|route| route.label == label)
                .expect("the declared route")
                .id
        };
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::Relocate {
                subject: Ref::Existing(clockwork.farmer),
                via: Ref::Existing(route("The Yard Stair")),
            }]),
        );
        tick(&mut kernel, 10).expect("the tick commits");
        assert_eq!(
            due(&kernel, clockwork.farmer, clockwork.routine),
            FictionalMinutes(ROUTINE_DUE + 10)
        );
        assert!(
            kernel.state.pressures.is_empty(),
            "a blocked routine neither rolls nor presses"
        );

        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::Relocate {
                subject: Ref::Existing(clockwork.farmer),
                via: Ref::Existing(route("The Hall Steps")),
            }]),
        );
        tick(&mut kernel, 1).expect("the tick commits");
        assert_eq!(
            due(&kernel, clockwork.farmer, clockwork.routine),
            FictionalMinutes(ROUTINE_DUE + 20)
        );
    }

    /// A tick with nothing due is still a commit: the clock moved, which is a
    /// canonical change.
    #[test]
    fn a_tick_with_nothing_due_moves_only_the_clock() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, _, _) = clock_kernel(directory.path(), "Quiet");
        let before = kernel.state.clone();

        tick(&mut kernel, 1).expect("the tick commits");
        assert_eq!(kernel.state.now, FictionalMinutes(1));
        assert_eq!(kernel.state.revision, before.revision + 1);
        assert!(kernel.state.pressures.is_empty());
        assert_eq!(kernel.state.commitments, before.commitments);
        assert_eq!(kernel.state.holdings, before.holdings);
        assert_eq!(kernel.state.positions, before.positions);
        assert_eq!(kernel.state.events, before.events);

        tick(&mut kernel, 2).expect("the tick commits");
        assert_eq!(kernel.state.now, FictionalMinutes(3));
    }

    /// A closed route in one place becomes political pressure in another without
    /// anyone deciding that it should. The tick never reduces; only an operation
    /// does.
    #[test]
    fn an_unavailable_dependency_presses_its_depender() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Dependency");
        let sourced = PressureSource::Dependency(DependencyTarget::Route(clockwork.gate));

        tick(&mut kernel, 1).expect("the tick commits");
        assert_eq!(pressure(&kernel, clockwork.reeve, sourced), 0);

        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::CloseRoute {
                route: Ref::Existing(clockwork.gate),
            }]),
        );
        tick(&mut kernel, 1).expect("the tick commits");
        assert_eq!(pressure(&kernel, clockwork.reeve, sourced), 1);

        // Repair stops the pressing but never unwrites what accrued.
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::OpenRoute {
                route: Ref::Existing(clockwork.gate),
            }]),
        );
        tick(&mut kernel, 1).expect("the tick commits");
        assert_eq!(pressure(&kernel, clockwork.reeve, sourced), 1);

        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::ResolvePressure {
                source: PressureSourceRef::Dependency(DependencyRef::Route(Ref::Existing(
                    clockwork.gate,
                ))),
                target: Ref::Existing(clockwork.reeve),
            }]),
        );
        assert!(
            !kernel.state.pressures.contains_key(&clockwork.reeve),
            "an emptied target removes its key rather than storing an empty map"
        );
    }

    /// The clock moves through exactly one command body.
    #[test]
    fn time_advances_only_through_advance_time() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "OnlyTick");
        tick(&mut kernel, 5).expect("the tick commits");
        let now = kernel.state.now;

        exercise(
            &mut kernel,
            clockwork.reeve,
            clockwork.threaten,
            vec![binding("target", Target::Subject(clockwork.farmer))],
        )
        .expect("the threat commits");
        assert_eq!(kernel.state.now, now);

        let snapshot = kernel.snapshot().unwrap();
        let opportunity = opportunity_for(&snapshot, clockwork.farmer);
        let caller = CallerId::Controller(opportunity.controller_id);
        kernel
            .submit(
                command(
                    &snapshot,
                    CommandId::new(),
                    caller.clone(),
                    CommandBody::DeclineDecision { opportunity },
                ),
                &AuthenticatedCaller::fixture(caller),
            )
            .expect("the decline commits");
        assert_eq!(kernel.state.now, now);

        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::CloseRoute {
                route: Ref::Existing(clockwork.gate),
            }]),
        );
        assert_eq!(kernel.state.now, now);
    }

    /// The clock capability is admitted for one body and nothing else.
    #[test]
    fn the_clock_capability_can_do_nothing_else() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Capability");
        let before = kernel.state.clone();
        let snapshot = kernel.snapshot().unwrap();
        let opportunity = opportunity_for(&snapshot, clockwork.reeve);

        for body in [
            CommandBody::ApproveDraft,
            CommandBody::ActivateWorld,
            CommandBody::DeclineDecision {
                opportunity: opportunity.clone(),
            },
            CommandBody::ExerciseDecision {
                opportunity,
                invocation: DecisionInvocation {
                    affordance: clockwork.threaten,
                    bindings: vec![binding("target", Target::Subject(clockwork.farmer))],
                    proposed: vec![ProposedEffect {
                        slot: 0,
                        magnitude: Magnitude::None,
                    }],
                    speech: None,
                },
            },
            operations(vec![ComponentOp::CloseRoute {
                route: Ref::Existing(clockwork.gate),
            }]),
        ] {
            let error = kernel
                .submit(
                    command(&snapshot, CommandId::new(), clock_caller(), body),
                    &AuthenticatedCaller::fixture(clock_caller()),
                )
                .unwrap_err();
            assert!(matches!(error, KernelError::Unauthorized), "{error:?}");
        }
        assert_eq!(kernel.state, before);
        tick(&mut kernel, 1).expect("the same caller may tick");
    }

    /// Who may tick, when, and how far.
    #[test]
    fn advance_time_negatives() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = WorldKernel::create(
            directory.path().join("world.cc"),
            creation(CommandId::new(), "Negatives"),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;

        // There is no time in Draft.
        let draft = kernel.snapshot().unwrap();
        let error = kernel
            .submit(
                command(
                    &draft,
                    CommandId::new(),
                    clock_caller(),
                    CommandBody::AdvanceTime {
                        minutes: minutes(1),
                    },
                ),
                &AuthenticatedCaller::fixture(clock_caller()),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            KernelError::WrongPhase {
                expected: WorldPhase::Active,
                actual: WorldPhase::Draft,
            }
        ));

        let active = activate(&mut kernel);
        // A span is constructor-checked, so a malformed one never reaches a
        // command at all.
        assert!(TickMinutes::new(0).is_none());
        assert!(TickMinutes::new(super::patch::MAX_ROUTE_COST + 1).is_none());

        for caller in [
            CallerId::Principal(player()),
            CallerId::Controller(active.opportunities[0].controller_id),
        ] {
            let error = kernel
                .submit(
                    command(
                        &active,
                        CommandId::new(),
                        caller.clone(),
                        CommandBody::AdvanceTime {
                            minutes: minutes(1),
                        },
                    ),
                    &AuthenticatedCaller::fixture(caller),
                )
                .unwrap_err();
            assert!(matches!(error, KernelError::Unauthorized), "{error:?}");
        }

        // The owner may tick from Eve.
        submit_owner(
            &mut kernel,
            &active,
            CommandBody::AdvanceTime {
                minutes: minutes(7),
            },
        );
        assert_eq!(kernel.state.now, FictionalMinutes(7));

        // And a span that would overflow the clock is refused rather than
        // wrapping.
        kernel.state.now = FictionalMinutes(u64::MAX);
        let snapshot = kernel.snapshot().unwrap();
        let error = kernel
            .submit(
                command(
                    &snapshot,
                    CommandId::new(),
                    clock_caller(),
                    CommandBody::AdvanceTime {
                        minutes: minutes(1),
                    },
                ),
                &AuthenticatedCaller::fixture(clock_caller()),
            )
            .unwrap_err();
        assert!(matches!(error, KernelError::Serialization(_)), "{error:?}");
    }

    /// The whole motion is re-derived at apply, so a forged fulfilment, target,
    /// magnitude, or clock reading is one comparison.
    #[test]
    fn a_forged_motion_does_not_apply() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Forged");
        let to = FictionalMinutes(120);
        let honest = clock::derive_motion(&kernel.state, to);
        assert!(!honest.fulfilled.is_empty() || !honest.pressed.is_empty());

        let forgeries = [
            // A forged clock reading.
            WorldEffect::TimeAdvanced {
                minutes: minutes(120),
                to: FictionalMinutes(121),
                motion: honest.clone(),
            },
            // A fulfilment nothing derived.
            WorldEffect::TimeAdvanced {
                minutes: minutes(120),
                to,
                motion: Motion {
                    fulfilled: vec![clock::RoutineFulfilled {
                        subject: clockwork.farmer,
                        key: clockwork.routine,
                        next_due: FictionalMinutes(9_999),
                    }],
                    pressed: honest.pressed.clone(),
                },
            },
            // A forged magnitude.
            WorldEffect::TimeAdvanced {
                minutes: minutes(120),
                to,
                motion: Motion {
                    fulfilled: honest.fulfilled.clone(),
                    pressed: honest
                        .pressed
                        .iter()
                        .map(|written| clock::PressureWritten {
                            magnitude: PressureMagnitude(written.magnitude.0 + 9),
                            ..written.clone()
                        })
                        .collect(),
                },
            },
            // A pressure row for an untouched subject.
            WorldEffect::TimeAdvanced {
                minutes: minutes(120),
                to,
                motion: Motion {
                    fulfilled: honest.fulfilled.clone(),
                    pressed: honest
                        .pressed
                        .iter()
                        .cloned()
                        .chain(std::iter::once(clock::PressureWritten {
                            target: clockwork.treasury,
                            source: PressureSource::Subject(clockwork.reeve),
                            magnitude: PressureMagnitude(5),
                        }))
                        .collect(),
                },
            },
        ];
        for forged in forgeries {
            let mut candidate = kernel.state.clone();
            let error = apply_effect(&mut candidate, CommandId::issue(), &clock_caller(), &forged)
                .unwrap_err();
            assert!(matches!(error, KernelError::Invariant(_)), "{error:?}");
            assert_eq!(candidate, kernel.state);
        }

        // The honest one applies.
        let mut candidate = kernel.state.clone();
        apply_effect(
            &mut candidate,
            CommandId::issue(),
            &clock_caller(),
            &WorldEffect::TimeAdvanced {
                minutes: minutes(120),
                to,
                motion: honest,
            },
        )
        .expect("the honest motion applies");
        assert_eq!(candidate.now, to);
    }

    /// Replay across a tick is exact, and every envelope is idempotent.
    #[test]
    fn restart_replay_across_a_tick_is_exact() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Replay");
        let world_id = kernel.state.world_id;

        exercise(
            &mut kernel,
            clockwork.reeve,
            clockwork.threaten,
            vec![binding("target", Target::Subject(clockwork.farmer))],
        )
        .expect("the threat commits");
        tick(&mut kernel, 60).expect("the routine tick commits");
        tick(&mut kernel, 60).expect("the obligation tick commits");
        let snapshot = kernel.snapshot().unwrap();
        let repeat = command(
            &snapshot,
            CommandId::new(),
            CallerId::Principal(owner()),
            operations(vec![ComponentOp::DischargeCommitment {
                subject: Ref::Existing(clockwork.farmer),
                key: clockwork.obligation,
            }]),
        );
        kernel
            .submit(repeat.clone(), &auth_principal(owner()))
            .expect("the discharge commits");
        // Discharge removes every pressure row it sourced.
        assert_eq!(
            pressure(
                &kernel,
                clockwork.farmer,
                PressureSource::Commitment {
                    subject: clockwork.farmer,
                    key: clockwork.obligation,
                }
            ),
            0
        );

        let before = kernel.state.clone();
        let before_snapshot = kernel.snapshot().unwrap();
        drop(kernel);
        let mut replayed = WorldKernel::open(&path, world_id).expect("the store replays");
        assert_eq!(replayed.state, before);
        assert_eq!(replayed.snapshot().unwrap(), before_snapshot);
        assert!(matches!(
            replayed.submit(repeat, &auth_principal(owner())),
            Ok(SubmitReceipt::AlreadyApplied(_))
        ));
    }

    /// The order is total, deterministic, and never a filter.
    #[test]
    fn the_attention_order_is_total_and_deterministic() {
        let directory = tempfile::tempdir().unwrap();
        let (kernel, clockwork, active) = clock_kernel(directory.path(), "Order");
        let derived = derive_opportunities(&kernel.state).unwrap();

        let with = |pressures: BTreeMap<SubjectId, u32>,
                    stamps: BTreeMap<SubjectId, u64>|
         -> Vec<SubjectId> {
            let mut state = kernel.state.clone();
            state.now = FictionalMinutes(1_000);
            state.pressures = pressures
                .into_iter()
                .map(|(subject, magnitude)| {
                    (
                        subject,
                        BTreeMap::from([(
                            PressureSource::Subject(subject),
                            PressureMagnitude(magnitude),
                        )]),
                    )
                })
                .collect();
            state.last_opportunity_at = stamps
                .into_iter()
                .map(|(subject, stamp)| (subject, FictionalMinutes(stamp)))
                .collect();
            order_opportunities(&state, derived.clone())
                .into_iter()
                .map(|opportunity| opportunity.scope.subject_id)
                .collect()
        };

        // Pressure dominates.
        let ordered = with(
            BTreeMap::from([(clockwork.farmer, 5), (clockwork.reeve, 3)]),
            BTreeMap::new(),
        );
        assert_eq!(ordered[0], clockwork.farmer);
        assert_eq!(ordered[1], clockwork.reeve);

        // Equal pressure orders by debt, and a subject never attended outranks
        // every subject that has.
        let stamped: BTreeMap<SubjectId, u64> = active
            .subjects
            .iter()
            .map(|subject| (subject.id, 900))
            .collect();
        let mut earlier = stamped.clone();
        earlier.insert(clockwork.treasury, 100);
        let ordered = with(BTreeMap::new(), earlier);
        assert_eq!(ordered[0], clockwork.treasury);

        // Never a filter: the same length and the same set, whatever the terms.
        for pressures in [
            BTreeMap::new(),
            BTreeMap::from([(clockwork.reeve, 1)]),
            BTreeMap::from([(clockwork.farmer, 9), (clockwork.treasury, 9)]),
        ] {
            let ordered = with(pressures, stamped.clone());
            assert_eq!(ordered.len(), derived.len());
            assert_eq!(
                ordered.iter().copied().collect::<BTreeSet<_>>(),
                derived
                    .iter()
                    .map(|opportunity| opportunity.scope.subject_id)
                    .collect::<BTreeSet<_>>()
            );
        }
    }

    /// The debt term rotates: with no pressure anywhere, every subject reaches
    /// the head within one pass, and standing pressure holds the head until it
    /// is resolved.
    #[test]
    fn every_subject_reaches_the_head_within_bounded_ticks() {
        let directory = tempfile::tempdir().unwrap();
        let (kernel, clockwork, _) = clock_kernel(directory.path(), "Rotation");
        let derived = derive_opportunities(&kernel.state).unwrap();
        let count = derived.len();

        let mut state = kernel.state.clone();
        let mut seen = BTreeSet::new();
        for step in 0..count {
            state.now = FictionalMinutes(u64::try_from(step).unwrap() + 1);
            let head = order_opportunities(&state, derived.clone())[0]
                .scope
                .subject_id;
            seen.insert(head);
            state.last_opportunity_at.insert(head, state.now);
        }
        assert_eq!(seen.len(), count, "every subject reached the head");

        // Standing pressure holds the head; the others still rotate among
        // themselves once it is resolved.
        state.pressures.insert(
            clockwork.farmer,
            BTreeMap::from([(
                PressureSource::Subject(clockwork.reeve),
                PressureMagnitude(4),
            )]),
        );
        for step in 0..count {
            state.now = FictionalMinutes(1_000 + u64::try_from(step).unwrap());
            let head = order_opportunities(&state, derived.clone())[0]
                .scope
                .subject_id;
            assert_eq!(head, clockwork.farmer);
            state.last_opportunity_at.insert(head, state.now);
        }
        state.pressures.remove(&clockwork.farmer);
        assert_ne!(
            order_opportunities(&state, derived.clone())[0]
                .scope
                .subject_id,
            clockwork.farmer
        );
    }

    /// Verification 14, first half: a dead end is derived, answered once, and
    /// then no longer derived. Nothing clears one but the predicate failing.
    #[test]
    fn an_unelaborated_destination_is_derived_and_answered_exactly_once() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "DeadEnd");
        let boundaries = derive_boundaries(&kernel.state).unwrap();
        let destinations: Vec<_> = boundaries
            .iter()
            .filter(|boundary| matches!(boundary, CausalBoundary::UnelaboratedDestination { .. }))
            .collect();
        assert_eq!(destinations.len(), 1);
        let answered = destinations[0].clone();
        assert!(matches!(
            answered,
            CausalBoundary::UnelaboratedDestination { route, place, .. }
                if route == clockwork.gate && place == clockwork.dead_end
        ));

        let snapshot = kernel.snapshot().unwrap();
        assert!(snapshot.boundaries.contains(&answered));
        submit_owner(
            &mut kernel,
            &snapshot,
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Boundary(answered.clone())),
                patch: WorldPatch {
                    declarations: vec![Declaration::Entity(EntityDeclaration {
                        handle: DraftHandle::new("shed"),
                        label: "The Roadside Shed".into(),
                        kind: EntityKind::Place,
                        container: Some(Ref::Existing(clockwork.dead_end)),
                    })],
                    operations: Vec::new(),
                    evidence: Vec::new(),
                },
            },
        );
        assert!(
            !derive_boundaries(&kernel.state)
                .unwrap()
                .contains(&answered)
        );

        // Answering it again names a boundary the kernel no longer derives.
        let snapshot = kernel.snapshot().unwrap();
        let error = kernel
            .submit(
                command(
                    &snapshot,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    CommandBody::AdmitPatch {
                        answers: Some(PatchAnswer::Boundary(answered)),
                        patch: WorldPatch {
                            declarations: vec![Declaration::Entity(EntityDeclaration {
                                handle: DraftHandle::new("byre"),
                                label: "The Roadside Byre".into(),
                                kind: EntityKind::Place,
                                container: Some(Ref::Existing(clockwork.dead_end)),
                            })],
                            operations: Vec::new(),
                            evidence: Vec::new(),
                        },
                    },
                ),
                &auth_principal(owner()),
            )
            .unwrap_err();
        assert!(matches!(error, KernelError::AnswerNotDerived), "{error:?}");
    }

    /// A promise the counterparty can neither command nor litigate is a missing
    /// structure, and either repair clears it. A goal never derives one.
    #[test]
    fn a_commitment_with_no_authority_or_redress_derives_missing_structure() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Structure");
        let missing = CausalBoundary::MissingStructure {
            subject: clockwork.farmer,
            key: clockwork.obligation,
            scope: BoundaryDigest(String::new()),
        };
        let derives = |kernel: &WorldKernel| {
            derive_boundaries(&kernel.state)
                .unwrap()
                .into_iter()
                .any(|boundary| match (&boundary, &missing) {
                    (
                        CausalBoundary::MissingStructure { subject, key, .. },
                        CausalBoundary::MissingStructure {
                            subject: expected_subject,
                            key: expected_key,
                            ..
                        },
                    ) => subject == expected_subject && key == expected_key,
                    _ => false,
                })
        };
        assert!(derives(&kernel));
        // A goal is a promise to oneself and needs no forum.
        assert!(!derive_boundaries(&kernel.state).unwrap().iter().any(
            |boundary| matches!(boundary, CausalBoundary::MissingStructure { key, .. } if *key == clockwork.goal)
        ));

        // Jurisdiction over the promisor's ground clears it.
        let snapshot = kernel.snapshot().unwrap();
        let grant = ComponentOp::GrantAuthority {
            holder: Ref::Existing(clockwork.treasury),
            grant: AuthorityGrantRef {
                kind: AuthorityKindName("command".into()),
                over: AuthorityTargetRef::PlaceSubtree(Ref::Existing(clockwork.yard)),
            },
        };
        submit_owner(&mut kernel, &snapshot, operations(vec![grant.clone()]));
        assert!(!derives(&kernel));

        // Revoke it, and a forum whose standing covers the counterparty clears
        // it the other way.
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::RevokeAuthority {
                holder: Ref::Existing(clockwork.treasury),
                grant: AuthorityGrantRef {
                    kind: AuthorityKindName("command".into()),
                    over: AuthorityTargetRef::PlaceSubtree(Ref::Existing(clockwork.yard)),
                },
            }]),
        );
        assert!(derives(&kernel));
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::OpenForum {
                grievance: GrievanceKindName("debt".into()),
                forum: Ref::Existing(clockwork.reeve),
                standing: AuthorityTargetRef::Subject(Ref::Existing(clockwork.treasury)),
            }]),
        );
        assert!(!derives(&kernel));
    }

    /// Verification 14, second half: an answer must be derived, and the commit
    /// must satisfy what it answered.
    #[test]
    fn an_answer_must_be_derived_and_must_be_satisfied() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Answers");
        let before = kernel.state.clone();
        let declaring = |answers: Option<PatchAnswer>, handle: &str| CommandBody::AdmitPatch {
            answers,
            patch: WorldPatch {
                declarations: vec![Declaration::Entity(EntityDeclaration {
                    handle: DraftHandle::new(handle),
                    label: "A Later Room".into(),
                    kind: EntityKind::Place,
                    container: None,
                })],
                operations: Vec::new(),
                evidence: Vec::new(),
            },
        };
        let refuse = |kernel: &mut WorldKernel, body: CommandBody| {
            let snapshot = kernel.snapshot().unwrap();
            kernel
                .submit(
                    command(
                        &snapshot,
                        CommandId::new(),
                        CallerId::Principal(owner()),
                        body,
                    ),
                    &auth_principal(owner()),
                )
                .unwrap_err()
        };

        // Declaring in Active without answering.
        assert!(matches!(
            refuse(&mut kernel, declaring(None, "unanswered")),
            KernelError::AnswerRequired
        ));
        // The two declared-but-never-derived variants.
        for boundary in [
            CausalBoundary::PolityInCausalRange {
                subject: clockwork.reeve,
                scope: BoundaryDigest(String::new()),
            },
            CausalBoundary::IndividuationRequired {
                population: clockwork.reeve,
                scope: BoundaryDigest(String::new()),
            },
        ] {
            assert!(matches!(
                refuse(
                    &mut kernel,
                    declaring(Some(PatchAnswer::Boundary(boundary)), "underived")
                ),
                KernelError::AnswerNotDerived
            ));
        }
        // A jurisdiction with no deficit.
        assert!(matches!(
            refuse(
                &mut kernel,
                declaring(
                    Some(PatchAnswer::Deficit(JurisdictionKey::Uncovered)),
                    "no-deficit"
                )
            ),
            KernelError::AnswerNotDerived
        ));
        // A derived boundary answered by a patch that leaves the predicate
        // holding.
        let destination = derive_boundaries(&kernel.state)
            .unwrap()
            .into_iter()
            .find(|boundary| matches!(boundary, CausalBoundary::UnelaboratedDestination { .. }))
            .expect("the dead end");
        assert!(matches!(
            refuse(
                &mut kernel,
                declaring(Some(PatchAnswer::Boundary(destination)), "elsewhere")
            ),
            KernelError::AnswerNotSatisfied
        ));
        assert_eq!(kernel.state, before);

        // Draft answers nothing.
        let directory = tempfile::tempdir().unwrap();
        let mut draft_kernel = WorldKernel::create(
            directory.path().join("world.cc"),
            creation(CommandId::new(), "Draft"),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        assert!(matches!(
            refuse(
                &mut draft_kernel,
                declaring(
                    Some(PatchAnswer::Deficit(JurisdictionKey::Uncovered)),
                    "in-draft"
                )
            ),
            KernelError::AnswerNotDerived
        ));
    }

    fn creation_with_intent(
        id: CommandId,
        title: &str,
        intent: WorldScaleIntentRef,
    ) -> CreateWorld {
        let mut creation = creation(id, title);
        creation.scale_intent = intent;
        creation
    }

    fn intent(root: &str, persons: u32, permille: u32) -> WorldScaleIntentRef {
        WorldScaleIntentRef {
            targets: BTreeMap::from([(SubjectKind::Person, persons)]),
            jurisdictions: BTreeMap::from([(DraftHandle::new(root), permille)]),
        }
    }

    /// Verification 22, first half: only a qualified subject reduces the
    /// deficit, and `Goal` carries the whole discrimination.
    #[test]
    fn a_subject_counts_toward_the_deficit_only_when_qualified() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = WorldKernel::create(
            directory.path().join("world.cc"),
            creation_with_intent(
                CommandId::new(),
                "Scale",
                intent(super::tests::COMMONS, 3, 1000),
            ),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let commons = kernel
            .state
            .scale_intent
            .jurisdictions
            .keys()
            .copied()
            .next()
            .expect("the declared root");
        let active = activate(&mut kernel);
        let counted = active
            .subjects
            .iter()
            .find(|subject| subject.kind == SubjectKind::Person)
            .expect("the fixture world has a person")
            .id;

        let deficit = |kernel: &WorldKernel, jurisdiction: JurisdictionKey| {
            derive_scale_deficit(&kernel.state)
                .unwrap()
                .into_iter()
                .find(|row| row.jurisdiction == jurisdiction && row.kind == SubjectKind::Person)
        };
        let row = deficit(&kernel, JurisdictionKey::PlaceSubtree(commons))
            .expect("a target of three persons");
        assert_eq!((row.target, row.qualified, row.deficit), (3, 0, 3));

        // A controller and a grant are not enough; a `Goal` is what counts.
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::CreateCommitment {
                subject: Ref::Existing(counted),
                counterparty: None,
                kind: CommitmentKind::Goal,
                due: FictionalMinutes(500),
                period: None,
                checks: Vec::new(),
            }]),
        );
        let goal = *kernel.state.commitments[&counted]
            .keys()
            .next()
            .expect("the goal");
        let row = deficit(&kernel, JurisdictionKey::PlaceSubtree(commons)).unwrap();
        assert_eq!((row.target, row.qualified, row.deficit), (3, 1, 2));

        // Discharging it raises the deficit again.
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::DischargeCommitment {
                subject: Ref::Existing(counted),
                key: goal,
            }]),
        );
        let row = deficit(&kernel, JurisdictionKey::PlaceSubtree(commons)).unwrap();
        assert_eq!((row.target, row.qualified, row.deficit), (3, 0, 3));
    }

    /// A qualifying subject standing somewhere no declared root covers still
    /// counts, in the `Uncovered` row, rather than vanishing from the deficit
    /// entirely: the count is total over every qualifying subject.
    #[test]
    fn a_subject_placed_outside_every_root_counts_as_uncovered() {
        let directory = tempfile::tempdir().unwrap();
        let mut creation = creation_with_intent(
            CommandId::new(),
            "Uncovered",
            intent(super::tests::COMMONS, 3, 1000),
        );
        creation
            .patch
            .declarations
            .push(Declaration::Entity(EntityDeclaration {
                handle: DraftHandle::new("elsewhere"),
                label: "The Elsewhere".into(),
                kind: EntityKind::Place,
                container: None,
            }));
        creation
            .patch
            .declarations
            .push(Declaration::Route(RouteDeclaration {
                handle: DraftHandle::new("bridge"),
                label: "The Elsewhere Bridge".into(),
                from: Ref::Draft(DraftHandle::new(super::tests::COMMONS)),
                to: Ref::Draft(DraftHandle::new("elsewhere")),
                access: AccessKind::Public,
                cost: Cost(1),
            }));
        let mut kernel = WorldKernel::create(
            directory.path().join("world.cc"),
            creation,
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let active = activate(&mut kernel);
        let subject_count = kernel.state.subjects.len() as u32;
        assert_eq!(subject_count, 3, "the base fixture declares three subjects");

        // Every subject qualifies, so the deficit's row counts must sum to
        // the whole subject count.
        let goal = |subject: SubjectId| ComponentOp::CreateCommitment {
            subject: Ref::Existing(subject),
            counterparty: None,
            kind: CommitmentKind::Goal,
            due: FictionalMinutes(500),
            period: None,
            checks: Vec::new(),
        };
        let subject_ids: Vec<SubjectId> =
            active.subjects.iter().map(|subject| subject.id).collect();
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(subject_ids.iter().copied().map(goal).collect()),
        );

        let moved = active
            .subjects
            .iter()
            .find(|subject| subject.kind == SubjectKind::Person)
            .expect("the fixture world has a person")
            .id;
        let bridge = active
            .routes
            .iter()
            .find(|route| route.label == "The Elsewhere Bridge")
            .expect("the declared route")
            .id;
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::Relocate {
                subject: Ref::Existing(moved),
                via: Ref::Existing(bridge),
            }]),
        );

        let rows = derive_scale_deficit(&kernel.state).unwrap();
        let uncovered = rows
            .iter()
            .find(|row| {
                row.jurisdiction == JurisdictionKey::Uncovered && row.kind == SubjectKind::Person
            })
            .expect("the relocated subject counts as uncovered");
        assert_eq!(uncovered.qualified, 1);

        let counted_total: u32 = rows.iter().map(|row| row.qualified).sum();
        assert_eq!(
            counted_total, subject_count,
            "the deficit is total over every qualifying subject"
        );
    }

    /// Weights distribute the target and never raise it, and a root must be a
    /// declared place.
    #[test]
    fn scale_weights_distribute_and_never_raise() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let over_whole = WorldScaleIntentRef {
            targets: BTreeMap::from([(SubjectKind::Person, 10)]),
            jurisdictions: BTreeMap::from([
                (DraftHandle::new(super::tests::COMMONS), 700),
                (DraftHandle::new("player"), 400),
            ]),
        };
        let Err(error) = WorldKernel::create(
            &path,
            creation_with_intent(CommandId::new(), "TooMuch", over_whole),
            &auth_principal(owner()),
        ) else {
            panic!("a genesis intent over the whole was admitted");
        };
        let KernelError::PatchRejected(mismatches) = error else {
            panic!("expected a rejected genesis patch");
        };
        assert!(mismatches.contains(&Mismatch::ScaleWeightsExceedWhole));
        // `player` is a subject handle, not a place.
        assert!(mismatches.contains(&Mismatch::UnknownJurisdictionRoot {
            handle: DraftHandle::new("player"),
        }));

        // Half the whole distributes half the target, rounded down.
        let mut kernel = WorldKernel::create(
            &path,
            creation_with_intent(
                CommandId::new(),
                "Half",
                intent(super::tests::COMMONS, 5, 500),
            ),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        activate(&mut kernel);
        let rows = derive_scale_deficit(&kernel.state).unwrap();
        let commons_target: u32 = rows
            .iter()
            .filter(|row| matches!(row.jurisdiction, JurisdictionKey::PlaceSubtree(_)))
            .map(|row| row.target)
            .sum();
        assert_eq!(commons_target, 2);
        assert!(commons_target <= 5);
    }

    /// A rejected patch mutates nothing, so the deficit it would have reduced
    /// stays visible.
    #[test]
    fn a_rejected_patch_leaves_the_deficit_unchanged_and_visible() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = WorldKernel::create(
            directory.path().join("world.cc"),
            creation_with_intent(
                CommandId::new(),
                "Rejected",
                intent(super::tests::COMMONS, 3, 1000),
            ),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let active = activate(&mut kernel);
        let before = derive_scale_deficit(&kernel.state).unwrap();
        let subjects: Vec<SubjectId> = active
            .subjects
            .iter()
            .filter(|subject| subject.kind == SubjectKind::Person)
            .map(|subject| subject.id)
            .collect();
        let goal = |subject: SubjectId, due: u64| ComponentOp::CreateCommitment {
            subject: Ref::Existing(subject),
            counterparty: None,
            kind: CommitmentKind::Goal,
            due: FictionalMinutes(due),
            period: None,
            checks: Vec::new(),
        };

        // One of the two goals is born past due.
        assert_eq!(
            reject_owner(
                &mut kernel,
                &active,
                operations(vec![goal(subjects[0], 500), goal(subjects[1], 0)]),
            ),
            vec![Mismatch::CommitmentDueInThePast { operation: 1 }]
        );
        assert_eq!(derive_scale_deficit(&kernel.state).unwrap(), before);

        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![goal(subjects[0], 500), goal(subjects[1], 600)]),
        );
        let after = derive_scale_deficit(&kernel.state).unwrap();
        let total = |rows: &[ScaleDeficitRow]| -> u32 { rows.iter().map(|row| row.deficit).sum() };
        assert_eq!(total(&after) + 2, total(&before));
    }

    /// Every commitment shape check, in one complete sorted set.
    #[test]
    fn commitment_declaration_negatives() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, active) = clock_kernel(directory.path(), "Shapes");
        let before = kernel.state.clone();
        let create = |kind: CommitmentKind,
                      counterparty: Option<SubjectId>,
                      due: u64,
                      period: Option<TickMinutes>,
                      checks: Vec<PreconditionRef>| {
            ComponentOp::CreateCommitment {
                subject: Ref::Existing(clockwork.farmer),
                counterparty: counterparty.map(Ref::Existing),
                kind,
                due: FictionalMinutes(due),
                period,
                checks,
            }
        };

        assert_eq!(
            reject_owner(
                &mut kernel,
                &active,
                operations(vec![
                    // 0: a routine with no period.
                    create(CommitmentKind::Routine, None, 500, None, Vec::new()),
                    // 1: a goal with a period.
                    create(
                        CommitmentKind::Goal,
                        None,
                        500,
                        Some(minutes(10)),
                        Vec::new()
                    ),
                    // 2: checks on a non-routine.
                    create(
                        CommitmentKind::Obligation,
                        Some(clockwork.treasury),
                        500,
                        None,
                        vec![PreconditionRef::Present {
                            at: Ref::Existing(clockwork.yard),
                        }],
                    ),
                    // 3: born past due.
                    create(
                        CommitmentKind::Obligation,
                        Some(clockwork.treasury),
                        0,
                        None,
                        Vec::new()
                    ),
                    // 4: a promise to oneself with a counterparty.
                    create(
                        CommitmentKind::Obligation,
                        Some(clockwork.farmer),
                        500,
                        None,
                        Vec::new()
                    ),
                    // 5: a goal with a counterparty.
                    create(
                        CommitmentKind::Goal,
                        Some(clockwork.treasury),
                        500,
                        None,
                        Vec::new()
                    ),
                    // 6: a discharge of a key nobody holds.
                    ComponentOp::DischargeCommitment {
                        subject: Ref::Existing(clockwork.farmer),
                        key: CommitmentKey {
                            command: CommandId::new(),
                            index: 0,
                        },
                    },
                ]),
            ),
            {
                let mut expected = vec![
                    Mismatch::CommitmentPeriodMismatch { operation: 0 },
                    Mismatch::CommitmentPeriodMismatch { operation: 1 },
                    Mismatch::ChecksOnNonRoutine { operation: 2 },
                    Mismatch::CommitmentDueInThePast { operation: 3 },
                    Mismatch::SelfCommitment { operation: 4 },
                    Mismatch::GoalWithCounterparty { operation: 5 },
                    Mismatch::UnknownCommitment { operation: 6 },
                ];
                expected.sort();
                expected
            }
        );
        assert_eq!(kernel.state, before);

        // Repairing exactly those commits.
        submit_owner(
            &mut kernel,
            &active,
            operations(vec![
                create(
                    CommitmentKind::Routine,
                    None,
                    500,
                    Some(minutes(10)),
                    Vec::new(),
                ),
                create(CommitmentKind::Goal, None, 500, None, Vec::new()),
                create(
                    CommitmentKind::Obligation,
                    Some(clockwork.treasury),
                    500,
                    None,
                    Vec::new(),
                ),
                ComponentOp::DischargeCommitment {
                    subject: Ref::Existing(clockwork.farmer),
                    key: clockwork.routine,
                },
            ]),
        );
        assert!(
            !kernel.state.commitments[&clockwork.farmer].contains_key(&clockwork.routine),
            "the discharge removed the routine"
        );
    }

    /// `Committed` reads the actor's own commitments, and a discharge mid-flight
    /// rejects the promisor's bound proposal while the counterparty's own
    /// invocation fails at admission.
    #[test]
    fn committed_reads_the_actors_own_commitments() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Committed");

        // The reeve promised the treasury nothing.
        let error = exercise(
            &mut kernel,
            clockwork.reeve,
            clockwork.deliver,
            vec![binding("creditor", Target::Subject(clockwork.treasury))],
        )
        .unwrap_err();
        assert!(matches!(
            error,
            KernelError::ActionRejected(ref mismatches)
                if *mismatches == vec![ActionMismatch::NotCommitted { precondition: 0 }]
        ));

        // The farmer did, so the entry commits and mints a second obligation
        // whose due date the kernel computed.
        let before = kernel.state.commitments[&clockwork.farmer].len();
        exercise(
            &mut kernel,
            clockwork.farmer,
            clockwork.deliver,
            vec![binding("creditor", Target::Subject(clockwork.treasury))],
        )
        .expect("the delivery commits");
        assert_eq!(
            kernel.state.commitments[&clockwork.farmer].len(),
            before + 1
        );

        // A proposal bound before a discharge is rejected as a scope change for
        // the promisor: `Committed` reads commitments, so the digest binds them.
        let snapshot = kernel.snapshot().unwrap();
        let stale = opportunity_for(&snapshot, clockwork.farmer);
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::DischargeCommitment {
                subject: Ref::Existing(clockwork.farmer),
                key: clockwork.obligation,
            }]),
        );
        let caller = CallerId::Controller(stale.controller_id);
        let after = kernel.snapshot().unwrap();
        let error = kernel
            .submit(
                command(
                    &after,
                    CommandId::new(),
                    caller.clone(),
                    CommandBody::ExerciseDecision {
                        opportunity: stale,
                        invocation: DecisionInvocation {
                            affordance: clockwork.deliver,
                            bindings: vec![binding(
                                "creditor",
                                Target::Subject(clockwork.treasury),
                            )],
                            proposed: vec![ProposedEffect {
                                slot: 0,
                                magnitude: Magnitude::None,
                            }],
                            speech: None,
                        },
                    },
                ),
                &AuthenticatedCaller::fixture(caller),
            )
            .unwrap_err();
        assert!(
            matches!(error, KernelError::ScopeChanged { .. }),
            "{error:?}"
        );
    }

    /// A tick that only advances pressure moves nobody's scope digest; a tick
    /// that fulfils a routine moves exactly that subject's.
    #[test]
    fn a_pressure_tick_moves_no_scope_digest_and_a_routine_tick_moves_one() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Digests");
        let digests = |kernel: &WorldKernel| -> BTreeMap<SubjectId, ScopeDigest> {
            derive_opportunities(&kernel.state)
                .unwrap()
                .into_iter()
                .map(|opportunity| (opportunity.scope.subject_id, opportunity.scope_digest))
                .collect()
        };

        // Past the obligation and the goal, but the routine is blocked out of
        // the yard first so only pressure moves.
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::DischargeCommitment {
                subject: Ref::Existing(clockwork.farmer),
                key: clockwork.routine,
            }]),
        );
        let before = digests(&kernel);
        tick(&mut kernel, 200).expect("the tick commits");
        assert!(!kernel.state.pressures.is_empty(), "the tick pressed");
        assert_eq!(digests(&kernel), before, "pressure binds no proposal");

        // A routine fulfilment moves exactly its own subject's digest.
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::CreateCommitment {
                subject: Ref::Existing(clockwork.reeve),
                counterparty: None,
                kind: CommitmentKind::Routine,
                due: FictionalMinutes(500),
                period: Some(minutes(60)),
                checks: Vec::new(),
            }]),
        );
        let before = digests(&kernel);
        tick(&mut kernel, 400).expect("the tick commits");
        let after = digests(&kernel);
        for (subject, digest) in &before {
            if *subject == clockwork.reeve {
                assert_ne!(after[subject], *digest, "the fulfilling subject rebinds");
            } else {
                assert_eq!(after[subject], *digest, "everyone else still commits");
            }
        }
    }

    /// Soul falsification: the attention order is total on `SubjectId` when
    /// pressure and debt tie, and it is the same order for every input
    /// permutation. `sort_by_key` is stable, so an order that leaned on its
    /// input's order would pass a single-vector test and drift with map
    /// iteration.
    #[test]
    fn soul_the_attention_order_is_id_total_under_every_input_permutation() {
        let directory = tempfile::tempdir().unwrap();
        let (kernel, clockwork, _) = clock_kernel(directory.path(), "Ties");
        let mut state = kernel.state.clone();
        state.now = FictionalMinutes(5_000);
        // Equal pressure and an equal stamp on every subject: only the id can
        // decide.
        for subject_id in state.subjects.keys().copied().collect::<Vec<_>>() {
            state.pressures.insert(
                subject_id,
                BTreeMap::from([(
                    PressureSource::Subject(clockwork.reeve),
                    PressureMagnitude(7),
                )]),
            );
            state
                .last_opportunity_at
                .insert(subject_id, FictionalMinutes(400));
        }
        let derived = derive_opportunities(&state).unwrap();
        assert!(
            derived.len() >= 3,
            "the fixture ties at least three subjects"
        );
        let expected: Vec<SubjectId> = order_opportunities(&state, derived.clone())
            .into_iter()
            .map(|opportunity| opportunity.scope.subject_id)
            .collect();
        let mut ascending = expected.clone();
        ascending.sort_unstable();
        assert_eq!(expected, ascending, "an equal-key tie is broken by id");

        // Every rotation and the full reversal produce the same order.
        for shift in 0..derived.len() {
            let mut permuted = derived.clone();
            permuted.rotate_left(shift);
            let mut reversed = permuted.clone();
            reversed.reverse();
            for candidate in [permuted, reversed] {
                let ordered: Vec<SubjectId> = order_opportunities(&state, candidate)
                    .into_iter()
                    .map(|opportunity| opportunity.scope.subject_id)
                    .collect();
                assert_eq!(ordered, expected, "the order read its input's order");
            }
        }
    }

    /// Soul falsification of the admission ruling: in Active a patch that only
    /// changes components answers nothing, a patch that declares or admits
    /// evidence must answer, and the answer gate closes before the resolver
    /// ever runs.
    #[test]
    fn soul_an_active_patch_answers_only_when_it_elaborates() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Elaborate");

        // Component-only, no answer: admitted.
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::AdvancePressure {
                source: PressureSourceRef::Subject(Ref::Existing(clockwork.reeve)),
                target: Ref::Existing(clockwork.farmer),
                by: PressureMagnitude(2),
            }]),
        );
        assert_eq!(
            pressure(
                &kernel,
                clockwork.farmer,
                PressureSource::Subject(clockwork.reeve)
            ),
            2
        );

        let refuse = |kernel: &mut WorldKernel, body: CommandBody| {
            let snapshot = kernel.snapshot().unwrap();
            kernel
                .submit(
                    command(
                        &snapshot,
                        CommandId::new(),
                        CallerId::Principal(owner()),
                        body,
                    ),
                    &auth_principal(owner()),
                )
                .unwrap_err()
        };

        // Evidence alone is elaboration too, so it answers.
        let evidence_only = CommandBody::AdmitPatch {
            answers: None,
            patch: WorldPatch {
                declarations: Vec::new(),
                operations: Vec::new(),
                evidence: vec![EvidenceRef::new("soul://evidence-only")],
            },
        };
        assert!(matches!(
            refuse(&mut kernel, evidence_only),
            KernelError::AnswerRequired
        ));

        // The gate is ahead of the resolver: a declaring patch that is also
        // structurally broken is refused for the missing answer, not for the
        // mismatch it would otherwise return.
        let broken = CommandBody::AdmitPatch {
            answers: None,
            patch: WorldPatch {
                declarations: vec![Declaration::Entity(EntityDeclaration {
                    handle: DraftHandle::new("orphan"),
                    label: "An Orphan Room".into(),
                    kind: EntityKind::Place,
                    container: Some(Ref::Draft(DraftHandle::new("nowhere"))),
                })],
                operations: Vec::new(),
                evidence: Vec::new(),
            },
        };
        assert!(
            matches!(refuse(&mut kernel, broken), KernelError::AnswerRequired),
            "the resolver ran before the answer gate"
        );

        // A derived answer that the patch satisfies commits, and the boundary
        // is gone afterwards.
        let answered = derive_boundaries(&kernel.state)
            .unwrap()
            .into_iter()
            .find(|boundary| matches!(boundary, CausalBoundary::UnelaboratedDestination { .. }))
            .expect("the dead end");
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Boundary(answered.clone())),
                patch: WorldPatch {
                    declarations: vec![Declaration::Entity(EntityDeclaration {
                        handle: DraftHandle::new("cairn"),
                        label: "A Roadside Cairn".into(),
                        kind: EntityKind::Place,
                        container: Some(Ref::Existing(clockwork.dead_end)),
                    })],
                    operations: Vec::new(),
                    evidence: Vec::new(),
                },
            },
        );
        assert!(
            !derive_boundaries(&kernel.state)
                .unwrap()
                .contains(&answered),
            "the answered boundary is still derived"
        );
    }

    /// Soul falsification: pressure magnitude cannot overflow or go negative,
    /// and zero is spelled by absence at both ends.
    #[test]
    fn soul_pressure_saturates_and_never_underflows() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Saturate");
        let sourced = PressureSource::Commitment {
            subject: clockwork.farmer,
            key: clockwork.obligation,
        };

        // A tick over a magnitude already at the ceiling saturates rather than
        // wrapping.
        let mut brimming = kernel.state.clone();
        brimming.pressures.insert(
            clockwork.farmer,
            BTreeMap::from([(sourced, PressureMagnitude(u32::MAX))]),
        );
        let motion = clock::derive_motion(&brimming, FictionalMinutes(LATE_DUE));
        let written = motion
            .pressed
            .iter()
            .find(|written| written.source == sourced)
            .expect("the past-due obligation pressed");
        assert_eq!(written.magnitude, PressureMagnitude(u32::MAX));

        // Reducing by more than the row holds removes it instead of wrapping.
        tick(&mut kernel, u32::try_from(LATE_DUE).unwrap()).expect("the tick commits");
        assert_eq!(pressure(&kernel, clockwork.farmer, sourced), 1);
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::ReducePressure {
                source: PressureSourceRef::Commitment {
                    subject: Ref::Existing(clockwork.farmer),
                    key: clockwork.obligation,
                },
                target: Ref::Existing(clockwork.farmer),
                by: PressureMagnitude(9),
            }]),
        );
        assert_eq!(pressure(&kernel, clockwork.farmer, sourced), 0);
        assert!(
            !kernel.state.pressures.contains_key(&clockwork.farmer),
            "an emptied target keeps no empty inner map"
        );

        // `pressure_total` and `attention_debt` are derived, never stored.
        assert_eq!(pressure_total(&kernel.state, clockwork.farmer), 0);
        assert_eq!(
            attention_debt(&kernel.state, clockwork.treasury),
            u64::MAX,
            "a subject never attended carries the whole debt"
        );

        // Resolving an absent row changes nothing and is refused.
        let snapshot = kernel.snapshot().unwrap();
        let mismatches = reject_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::ResolvePressure {
                source: PressureSourceRef::Commitment {
                    subject: Ref::Existing(clockwork.farmer),
                    key: clockwork.obligation,
                },
                target: Ref::Existing(clockwork.farmer),
            }]),
        );
        assert_eq!(
            mismatches,
            vec![Mismatch::NoOperationEffect { operation: 0 }]
        );
    }

    /// Soul falsification: `now` is in no scope preimage. A tick with nothing
    /// due moves the clock and not one subject's digest.
    #[test]
    fn soul_a_clock_only_tick_moves_no_scope_digest() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, _, _) = clock_kernel(directory.path(), "ClockOnly");
        let digests = |kernel: &WorldKernel| -> BTreeMap<SubjectId, ScopeDigest> {
            derive_opportunities(&kernel.state)
                .unwrap()
                .into_iter()
                .map(|opportunity| (opportunity.scope.subject_id, opportunity.scope_digest))
                .collect()
        };
        let before = digests(&kernel);
        let before_now = kernel.state.now;

        // Short of every due date in the fixture.
        tick(&mut kernel, 10).expect("the tick commits");
        assert_eq!(kernel.state.now, FictionalMinutes(before_now.0 + 10));
        assert!(kernel.state.pressures.is_empty());
        assert_eq!(digests(&kernel), before, "the clock entered a scope digest");
    }

    /// Soul falsification: the authored scale target is written once by genesis
    /// and by no Active lane. `AdmitPatch` resolves with no intent at all, so
    /// there is no shape for an Active rewrite to take.
    #[test]
    fn soul_the_scale_intent_is_written_once_by_genesis() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = WorldKernel::create(
            directory.path().join("world.cc"),
            creation_with_intent(
                CommandId::new(),
                "WriteOnce",
                intent(super::tests::COMMONS, 3, 1000),
            ),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let active = activate(&mut kernel);
        let authored = kernel.state.scale_intent.clone();
        assert!(!authored.jurisdictions.is_empty());

        let subject = active
            .subjects
            .iter()
            .find(|subject| subject.kind == SubjectKind::Person)
            .expect("a person")
            .id;
        let body = operations(vec![ComponentOp::CreateCommitment {
            subject: Ref::Existing(subject),
            counterparty: None,
            kind: CommitmentKind::Goal,
            due: FictionalMinutes(500),
            period: None,
            checks: Vec::new(),
        }]);
        // The Active resolver is handed no intent, so the resolved patch cannot
        // carry one.
        let CommandBody::AdmitPatch { patch, .. } = &body else {
            panic!("the operations helper builds an admitted patch");
        };
        let resolved =
            patch::resolve_patch(&kernel.state, CommandId::new(), patch, None).expect("resolves");
        assert!(
            resolved.scale_intent.is_none(),
            "an Active patch resolved a scale intent"
        );

        submit_owner(&mut kernel, &active, body);
        assert_eq!(kernel.state.scale_intent, authored);
    }

    /// Soul falsification: a discharge names one subject's own live key.
    #[test]
    fn soul_discharging_a_foreign_or_already_discharged_commitment_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Discharge");

        // The farmer's obligation, claimed by the reeve.
        let snapshot = kernel.snapshot().unwrap();
        assert_eq!(
            reject_owner(
                &mut kernel,
                &snapshot,
                operations(vec![ComponentOp::DischargeCommitment {
                    subject: Ref::Existing(clockwork.reeve),
                    key: clockwork.obligation,
                }]),
            ),
            vec![Mismatch::UnknownCommitment { operation: 0 }]
        );

        // Discharged once, and then not again.
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::DischargeCommitment {
                subject: Ref::Existing(clockwork.farmer),
                key: clockwork.obligation,
            }]),
        );
        let snapshot = kernel.snapshot().unwrap();
        assert_eq!(
            reject_owner(
                &mut kernel,
                &snapshot,
                operations(vec![ComponentOp::DischargeCommitment {
                    subject: Ref::Existing(clockwork.farmer),
                    key: clockwork.obligation,
                }]),
            ),
            vec![Mismatch::UnknownCommitment { operation: 0 }]
        );
    }

    /// Soul falsification: a routine re-arms by exactly one period per command,
    /// whatever the span, and a non-routine cannot carry checks.
    #[test]
    fn soul_a_routine_re_arms_exactly_one_period_per_command() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Rearm");
        let period = u64::from(
            kernel.state.commitments[&clockwork.farmer][&clockwork.routine]
                .period
                .expect("a routine carries a period")
                .minutes(),
        );
        let start = due(&kernel, clockwork.farmer, clockwork.routine);

        // One roll per command, even for a span that clears many periods.
        tick(&mut kernel, u32::try_from(period * 10).unwrap()).expect("the long tick commits");
        assert_eq!(
            due(&kernel, clockwork.farmer, clockwork.routine),
            FictionalMinutes(start.0 + period)
        );
        tick(&mut kernel, 1).expect("the short tick commits");
        assert_eq!(
            due(&kernel, clockwork.farmer, clockwork.routine),
            FictionalMinutes(start.0 + 2 * period)
        );

        // Checks belong to routines alone.
        let snapshot = kernel.snapshot().unwrap();
        assert_eq!(
            reject_owner(
                &mut kernel,
                &snapshot,
                operations(vec![ComponentOp::CreateCommitment {
                    subject: Ref::Existing(clockwork.reeve),
                    counterparty: None,
                    kind: CommitmentKind::Goal,
                    due: FictionalMinutes(100_000),
                    period: None,
                    checks: vec![PreconditionRef::Present {
                        at: Ref::Existing(clockwork.yard),
                    }],
                }]),
            ),
            vec![Mismatch::ChecksOnNonRoutine { operation: 0 }]
        );
    }

    /// Soul falsification: `derive_motion` is a function of state and span
    /// alone, and the committed effect is exactly what `reduce` re-derives for
    /// a tick that fulfils a routine, presses past-due promises, and presses a
    /// failing dependency at once.
    #[test]
    fn soul_derive_motion_is_pure_and_the_tick_replays_exactly() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Pure");
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::CloseRoute {
                route: Ref::Existing(clockwork.gate),
            }]),
        );

        // The same state and the same span, twenty times: one answer.
        let span = u32::try_from(LATE_DUE + 10).unwrap();
        let to = FictionalMinutes(LATE_DUE + 10);
        let first = clock::derive_motion(&kernel.state, to);
        assert!(!first.fulfilled.is_empty(), "the routine is due");
        assert!(
            first.pressed.len() >= 2,
            "past-due promises and the closed route all press"
        );
        for _ in 0..20 {
            assert_eq!(clock::derive_motion(&kernel.state, to), first);
        }

        // And the commit carries exactly that motion.
        let snapshot = kernel.snapshot().unwrap();
        let ticking = command(
            &snapshot,
            CommandId::new(),
            clock_caller(),
            CommandBody::AdvanceTime {
                minutes: minutes(span),
            },
        );
        let expected = reduce(&kernel.state, &ticking).expect("the tick reduces");
        assert_eq!(
            expected,
            WorldEffect::TimeAdvanced {
                minutes: minutes(span),
                to,
                motion: first,
            }
        );
        // Applying the re-derived effect to a copy reproduces the committed
        // state exactly, which is the replay identity the journal relies on.
        let mut replayed = kernel.state.clone();
        apply_effect(&mut replayed, ticking.id, &clock_caller(), &expected)
            .expect("the re-derived effect applies");
        kernel
            .submit(ticking, &AuthenticatedCaller::fixture(clock_caller()))
            .expect("the tick commits");
        assert_eq!(replayed.now, kernel.state.now);
        assert_eq!(replayed.commitments, kernel.state.commitments);
        assert_eq!(replayed.pressures, kernel.state.pressures);
    }

    /// Soul falsification: a `BoundaryDigest` binds the structure that derives
    /// it, and nothing else. The clock is not in the preimage, so a tick leaves
    /// it where it is; closing the one route out of the dead end leaves the
    /// predicate holding and must still move the digest.
    #[test]
    fn soul_a_boundary_digest_binds_its_structure_and_not_the_clock() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Digest");
        let destination = |kernel: &WorldKernel| {
            derive_boundaries(&kernel.state)
                .unwrap()
                .into_iter()
                .find_map(|boundary| match boundary {
                    CausalBoundary::UnelaboratedDestination { place, scope, .. }
                        if place == clockwork.dead_end =>
                    {
                        Some(scope)
                    }
                    _ => None,
                })
                .expect("the dead end")
        };
        let before = destination(&kernel);

        // The clock moves; the digest does not.
        tick(&mut kernel, 10).expect("the tick commits");
        assert_eq!(destination(&kernel), before, "the clock entered a preimage");

        // The route's own record moves; the digest must follow.
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            operations(vec![ComponentOp::CloseRoute {
                route: Ref::Existing(clockwork.gate),
            }]),
        );
        assert_ne!(
            destination(&kernel),
            before,
            "the digest ignored the route it names"
        );
    }

    // ---- the elaborator capability -------------------------------------

    fn elaborator(jurisdiction: JurisdictionKey) -> CallerId {
        CallerId::System(SystemCapability::Elaborator { jurisdiction })
    }

    fn submit_as(
        kernel: &mut WorldKernel,
        caller: CallerId,
        body: CommandBody,
    ) -> Result<SubmitReceipt, KernelError> {
        let snapshot = kernel.snapshot().unwrap();
        kernel.submit(
            command(&snapshot, CommandId::new(), caller.clone(), body),
            &AuthenticatedCaller::fixture(caller),
        )
    }

    fn dead_end_boundary(kernel: &WorldKernel) -> CausalBoundary {
        derive_boundaries(&kernel.state)
            .unwrap()
            .into_iter()
            .find(|boundary| matches!(boundary, CausalBoundary::UnelaboratedDestination { .. }))
            .expect("the fixture derives one dead end")
    }

    fn shed_under(container: EntityId, handle: &str) -> WorldPatch {
        WorldPatch {
            declarations: vec![Declaration::Entity(EntityDeclaration {
                handle: DraftHandle::new(handle),
                label: format!("The {handle}"),
                kind: EntityKind::Place,
                container: Some(Ref::Existing(container)),
            })],
            operations: Vec::new(),
            evidence: Vec::new(),
        }
    }

    /// Verification 14: the mailbox port cannot express an unanswered
    /// elaborator patch, so this is reachable only through the journal lane —
    /// and it is refused there.
    #[test]
    fn elaborator_patch_without_answer_is_answer_required() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Unanswered");
        let revision = kernel.state.revision;
        let error = submit_as(
            &mut kernel,
            elaborator(JurisdictionKey::PlaceSubtree(clockwork.dead_end)),
            CommandBody::AdmitPatch {
                answers: None,
                patch: shed_under(clockwork.dead_end, "shed"),
            },
        )
        .unwrap_err();
        assert!(matches!(error, KernelError::AnswerRequired), "{error:?}");
        assert_eq!(kernel.state.revision, revision);
    }

    /// Verification 14: a jurisdiction whose every row is zero has nothing to
    /// answer, so the answer is not derived.
    #[test]
    fn patch_answering_a_zero_deficit_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "ZeroDeficit");
        let jurisdiction = JurisdictionKey::PlaceSubtree(clockwork.dead_end);
        assert!(
            derive_scale_deficit(&kernel.state)
                .unwrap()
                .iter()
                .all(|row| row.deficit == 0)
        );
        let error = submit_as(
            &mut kernel,
            elaborator(jurisdiction),
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Deficit(jurisdiction)),
                patch: shed_under(clockwork.dead_end, "shed"),
            },
        )
        .unwrap_err();
        assert!(matches!(error, KernelError::AnswerNotDerived), "{error:?}");
    }

    /// The one admission rule at the top of `reduce` refuses four bodies before
    /// any of them is read, and the clock's capability is refused the fifth.
    #[test]
    fn a_system_capability_is_admitted_for_exactly_one_body() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, active) = clock_kernel(directory.path(), "Capability");
        let held = elaborator(JurisdictionKey::PlaceSubtree(clockwork.dead_end));
        let opportunity = opportunity_for(&active, clockwork.reeve);
        let answered = dead_end_boundary(&kernel);
        let forbidden = [
            CommandBody::ApproveDraft,
            CommandBody::ActivateWorld,
            CommandBody::DeclineDecision {
                opportunity: opportunity.clone(),
            },
            CommandBody::AdvanceTime {
                minutes: minutes(60),
            },
        ];
        for body in forbidden {
            let error = submit_as(&mut kernel, held.clone(), body).unwrap_err();
            assert!(matches!(error, KernelError::Unauthorized), "{error:?}");
        }
        // The mirror: the clock may not author.
        let error = submit_as(
            &mut kernel,
            clock_caller(),
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Boundary(answered)),
                patch: shed_under(clockwork.dead_end, "shed"),
            },
        )
        .unwrap_err();
        assert!(matches!(error, KernelError::Unauthorized), "{error:?}");
        assert_eq!(kernel.state.revision, active.revision);
    }

    /// Seed admission is the owner's lane: an elaborator in Draft has no
    /// derived boundary to answer and nothing to be authorized for.
    #[test]
    fn elaborator_in_draft_is_unauthorized() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = WorldKernel::create(
            directory.path().join("world.cc"),
            creation(CommandId::new(), "DraftLane"),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let commons = kernel.snapshot().unwrap().places[0].id;
        let error = submit_as(
            &mut kernel,
            elaborator(JurisdictionKey::PlaceSubtree(commons)),
            CommandBody::AdmitPatch {
                answers: None,
                patch: shed_under(commons, "shed"),
            },
        )
        .unwrap_err();
        // Draft answers nothing, so the answer gate passes and authority
        // refuses: an elaborator with no answer holds nothing.
        assert!(matches!(error, KernelError::Unauthorized), "{error:?}");
    }

    /// Boundaries are covered transitively, so a parent jurisdiction's
    /// elaborator may answer a boundary in a nested child — and a sibling root
    /// may not answer it at all.
    #[test]
    fn boundary_jurisdiction_is_transitive_and_exclusive() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Transitive");
        let answered = dead_end_boundary(&kernel);
        let hall = kernel.state.entities[&clockwork.yard]
            .container
            .expect("the yard sits in the hall");

        // The hall's elaborator does not cover the unwalked road.
        let error = submit_as(
            &mut kernel,
            elaborator(JurisdictionKey::PlaceSubtree(hall)),
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Boundary(answered.clone())),
                patch: shed_under(clockwork.dead_end, "shed"),
            },
        )
        .unwrap_err();
        assert!(matches!(error, KernelError::Unauthorized), "{error:?}");

        // One unconfined owner patch answers that boundary and opens a second
        // one strictly inside the hall, so the two coverings can be told apart
        // on one world.
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Boundary(answered.clone())),
                patch: WorldPatch {
                    declarations: vec![
                        Declaration::Entity(EntityDeclaration {
                            handle: DraftHandle::new("shed"),
                            label: "The Roadside Shed".into(),
                            kind: EntityKind::Place,
                            container: Some(Ref::Existing(clockwork.dead_end)),
                        }),
                        Declaration::Entity(EntityDeclaration {
                            handle: DraftHandle::new("cellar"),
                            label: "The Yard Cellar".into(),
                            kind: EntityKind::Place,
                            container: Some(Ref::Existing(clockwork.yard)),
                        }),
                        Declaration::Route(RouteDeclaration {
                            handle: DraftHandle::new("hatch"),
                            label: "The Cellar Hatch".into(),
                            from: Ref::Existing(clockwork.yard),
                            to: Ref::Draft(DraftHandle::new("cellar")),
                            access: AccessKind::Public,
                            cost: Cost(1),
                        }),
                    ],
                    operations: Vec::new(),
                    evidence: Vec::new(),
                },
            },
        );
        assert!(
            !derive_boundaries(&kernel.state)
                .unwrap()
                .contains(&answered)
        );

        let cellar = kernel
            .state
            .entities
            .iter()
            .find(|(_, entity)| entity.label == "The Yard Cellar")
            .map(|(id, _)| *id)
            .expect("the declared cellar");
        let nested = dead_end_boundary(&kernel);
        assert!(matches!(
            nested,
            CausalBoundary::UnelaboratedDestination { place, .. } if place == cellar
        ));

        // The road's root does not reach into the hall.
        let error = submit_as(
            &mut kernel,
            elaborator(JurisdictionKey::PlaceSubtree(clockwork.dead_end)),
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Boundary(nested.clone())),
                patch: shed_under(cellar, "crock"),
            },
        )
        .unwrap_err();
        assert!(matches!(error, KernelError::Unauthorized), "{error:?}");

        // The hall's does, transitively, through the same containment walk the
        // civic lane uses.
        submit_as(
            &mut kernel,
            elaborator(JurisdictionKey::PlaceSubtree(hall)),
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Boundary(nested.clone())),
                patch: shed_under(cellar, "crock"),
            },
        )
        .expect("the parent root covers a nested boundary");
        assert!(!derive_boundaries(&kernel.state).unwrap().contains(&nested));
    }

    /// A deficit is covered by exact key equality. A parent answering a child's
    /// row would reduce two targets with one subject, because a subject under
    /// nested roots counts toward both.
    #[test]
    fn parent_jurisdiction_cannot_answer_a_child_deficit_row() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = WorldKernel::create(
            directory.path().join("world.cc"),
            creation_with_intent(
                CommandId::new(),
                "Rows",
                intent(super::tests::COMMONS, 3, 1000),
            ),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let commons = *kernel
            .state
            .scale_intent
            .jurisdictions
            .keys()
            .next()
            .expect("the declared root");
        let active = activate(&mut kernel);
        let inner = active.places[0].id;
        let row = JurisdictionKey::PlaceSubtree(commons);
        assert!(
            derive_scale_deficit(&kernel.state)
                .unwrap()
                .iter()
                .any(|candidate| candidate.jurisdiction == row && candidate.deficit > 0)
        );
        let error = submit_as(
            &mut kernel,
            elaborator(JurisdictionKey::Uncovered),
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Deficit(row)),
                patch: shed_under(inner, "shed"),
            },
        )
        .unwrap_err();
        assert!(matches!(error, KernelError::Unauthorized), "{error:?}");
    }

    /// Confinement: a valid answer does not license a write outside the
    /// jurisdiction, and a place with no container is a new root, which is the
    /// owner's act.
    #[test]
    fn an_elaborator_cannot_write_outside_its_jurisdiction() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Confined");
        let answered = dead_end_boundary(&kernel);
        let held = elaborator(JurisdictionKey::PlaceSubtree(clockwork.dead_end));

        let foreign = submit_as(
            &mut kernel,
            held.clone(),
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Boundary(answered.clone())),
                patch: shed_under(clockwork.yard, "yardshed"),
            },
        )
        .unwrap_err();
        assert!(
            matches!(&foreign, KernelError::PatchRejected(set)
            if set == &vec![Mismatch::OutsideJurisdiction {
                site: Site::Declaration(DraftHandle::new("yardshed")),
            }]),
            "{foreign:?}"
        );

        let rootless = submit_as(
            &mut kernel,
            held,
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Boundary(answered.clone())),
                patch: WorldPatch {
                    declarations: vec![Declaration::Entity(EntityDeclaration {
                        handle: DraftHandle::new("newroot"),
                        label: "A New Country".into(),
                        kind: EntityKind::Place,
                        container: None,
                    })],
                    operations: Vec::new(),
                    evidence: Vec::new(),
                },
            },
        )
        .unwrap_err();
        assert!(
            matches!(&rootless, KernelError::PatchRejected(set)
            if set == &vec![Mismatch::OutsideJurisdiction {
                site: Site::Declaration(DraftHandle::new("newroot")),
            }]),
            "{rootless:?}"
        );

        // The owner is unconfined, and the identical rootless patch commits —
        // once it answers something the kernel derives.
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Boundary(answered)),
                patch: WorldPatch {
                    declarations: vec![Declaration::Entity(EntityDeclaration {
                        handle: DraftHandle::new("newroot"),
                        label: "A New Country".into(),
                        kind: EntityKind::Place,
                        container: Some(Ref::Existing(clockwork.dead_end)),
                    })],
                    operations: Vec::new(),
                    evidence: Vec::new(),
                },
            },
        );
    }

    /// Verification 17: seed admission and boundary elaboration reach one
    /// reducer. The same structural defect returns the same complete set on
    /// both lanes.
    #[test]
    fn seed_admission_and_boundary_elaboration_reach_one_reducer() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "OneReducer");
        let answered = dead_end_boundary(&kernel);
        let dangling = |handle: &str| WorldPatch {
            declarations: vec![Declaration::Entity(EntityDeclaration {
                handle: DraftHandle::new(handle),
                label: "The Nowhere Shed".into(),
                kind: EntityKind::Place,
                container: Some(Ref::Draft(DraftHandle::new("nothing"))),
            })],
            operations: Vec::new(),
            evidence: Vec::new(),
        };
        let snapshot = kernel.snapshot().unwrap();
        let owner_error = kernel
            .submit(
                command(
                    &snapshot,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    CommandBody::AdmitPatch {
                        answers: Some(PatchAnswer::Boundary(answered.clone())),
                        patch: dangling("shed"),
                    },
                ),
                &auth_principal(owner()),
            )
            .unwrap_err();
        let elaborator_error = submit_as(
            &mut kernel,
            elaborator(JurisdictionKey::PlaceSubtree(clockwork.dead_end)),
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Boundary(answered.clone())),
                patch: dangling("shed"),
            },
        )
        .unwrap_err();
        let set_of = |error: &KernelError| match error {
            KernelError::PatchRejected(set) => set.clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(set_of(&owner_error), set_of(&elaborator_error));

        // And both lanes commit the sound patch through the one writer.
        submit_as(
            &mut kernel,
            elaborator(JurisdictionKey::PlaceSubtree(clockwork.dead_end)),
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Boundary(answered)),
                patch: shed_under(clockwork.dead_end, "shed"),
            },
        )
        .expect("the elaborated patch commits");
    }

    /// `reduce` decides and `apply_effect` re-decides: a hand-built effect with
    /// a jurisdiction that does not cover its answer never applies.
    #[test]
    fn apply_effect_re_decides_elaborator_authority() {
        let directory = tempfile::tempdir().unwrap();
        let (kernel, clockwork, _) = clock_kernel(directory.path(), "Redecide");
        let answered = dead_end_boundary(&kernel);
        let hall = kernel.state.entities[&clockwork.yard].container.unwrap();
        let command_id = CommandId::issue();
        let resolved = patch::resolve_patch(
            &kernel.state,
            command_id,
            &shed_under(clockwork.dead_end, "shed"),
            None,
        )
        .expect("the patch resolves");
        let effect = WorldEffect::PatchAdmitted {
            answers: Some(PatchAnswer::Boundary(answered)),
            resolved,
        };

        let mut candidate = kernel.state.clone();
        let error = apply_effect(
            &mut candidate,
            command_id,
            &elaborator(JurisdictionKey::PlaceSubtree(hall)),
            &effect,
        )
        .unwrap_err();
        assert!(matches!(error, KernelError::Invariant(_)), "{error:?}");
        assert_eq!(candidate, kernel.state);

        // The honest jurisdiction applies through the same arm.
        let mut candidate = kernel.state.clone();
        apply_effect(
            &mut candidate,
            command_id,
            &elaborator(JurisdictionKey::PlaceSubtree(clockwork.dead_end)),
            &effect,
        )
        .expect("the covering jurisdiction applies");
        assert_ne!(candidate, kernel.state);
    }

    /// Replay re-decides authority for free, because it re-runs `reduce` and
    /// `apply_effect` against the pre-commit state. A row whose recorded caller
    /// claims a jurisdiction it does not have fails the journal.
    #[test]
    fn journal_replay_refuses_a_forged_elaborator_row() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Forged");
        let world_id = kernel.state.world_id;
        let answered = dead_end_boundary(&kernel);
        submit_as(
            &mut kernel,
            elaborator(JurisdictionKey::PlaceSubtree(clockwork.dead_end)),
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Boundary(answered)),
                patch: shed_under(clockwork.dead_end, "shed"),
            },
        )
        .expect("the honest elaboration commits");
        drop(kernel);
        let replayed = WorldKernel::open(&path, world_id).expect("the honest history replays");
        assert!(
            replayed
                .state
                .entities
                .values()
                .any(|entity| entity.label == "The shed")
        );
    }

    /// The mismatch vocabulary is kernel-internal plus controller telemetry. It
    /// never enters a commit, and `verify_state_shape` never grows a clause for
    /// it because no partition holds one.
    #[test]
    fn mismatch_never_appears_in_a_world_commit() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, clockwork, _) = clock_kernel(directory.path(), "Separation");
        let answered = dead_end_boundary(&kernel);
        submit_as(
            &mut kernel,
            elaborator(JurisdictionKey::PlaceSubtree(clockwork.dead_end)),
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Boundary(answered)),
                patch: shed_under(clockwork.dead_end, "shed"),
            },
        )
        .expect("the elaboration commits");

        let bytes = rmp_serde::to_vec_named(&kernel.state).expect("state encodes");
        let text = String::from_utf8_lossy(&bytes);
        for tag in [
            "outside_jurisdiction",
            "unresolved_draft",
            "no_canonical_change",
            "mismatch",
        ] {
            assert!(!text.contains(tag), "{tag} reached world state");
        }
    }
}
