//! Closed-patch resolution: the one owner of draft handles, reference kinds, and
//! canonical ID allocation for every admission lane.
//!
//! A patch names structure two ways and no other way: an exact canonical ID that
//! already keys a partition, or a draft handle declared in the same patch.
//! Resolution accumulates the complete mismatch set first and allocates only
//! after that set is empty, so a rejected patch never mints an ID.

use super::{
    AffordanceGrant, AffordanceId, AffordanceKind, CommandId, ControllerAssignment, ControllerId,
    DecisionScope, EntityId, NewController, SubjectId, SubjectKind, SubjectState, WorldId,
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EdgeKind {
    Route,
    Relation,
    Commitment,
    Pressure,
}

/// Namespace plus declared kind of a draft handle. A reference states the kind it
/// expects; resolution refuses a handle that answers with another one.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "namespace", rename_all = "snake_case")]
pub(crate) enum RefKind {
    Subject(SubjectKind),
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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "ref", content = "value", rename_all = "snake_case")]
pub(crate) enum Ref<Id> {
    Existing(Id),
    Draft(DraftHandle),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct EvidenceRef(String);

/// Pass 1 declares subjects and entities. `authority_scope` is the single
/// reference-bearing field: an institution's jurisdiction over a place.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubjectDeclaration {
    pub(crate) handle: DraftHandle,
    pub(crate) label: String,
    pub(crate) kind: SubjectKind,
    pub(crate) controller: NewController,
    pub(crate) affordances: BTreeSet<AffordanceKind>,
    pub(crate) authority_scope: Option<Ref<EntityId>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct EntityDeclaration {
    pub(crate) handle: DraftHandle,
    pub(crate) label: String,
    pub(crate) kind: EntityKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Declaration {
    Subject(SubjectDeclaration),
    Entity(EntityDeclaration),
}

/// Empty on purpose: a `Vec<ComponentOp>` can only be empty, so "no components
/// yet" is compile-enforced instead of runtime-checked.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum ComponentOp {}

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
        handle: DraftHandle,
        expected: RefKind,
    },
    WrongKind {
        handle: DraftHandle,
        expected: RefKind,
        actual: RefKind,
    },
    UnknownCanonical {
        handle: DraftHandle,
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
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct EntityRecord {
    pub(super) label: String,
    pub(super) kind: EntityKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct EdgeRecord {
    pub(super) kind: EdgeKind,
    pub(super) from: EntityId,
    pub(super) to: EntityId,
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
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct ResolvedEntity {
    pub(super) handle: DraftHandle,
    pub(super) entity_id: EntityId,
    pub(super) entity: EntityRecord,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct ResolvedPatch {
    pub(super) subjects: Vec<ResolvedSubject>,
    pub(super) entities: Vec<ResolvedEntity>,
    pub(super) evidence: Vec<EvidenceRef>,
}

const SUBJECT_NAMESPACE: &str = "ghostlight.id.subject.v1";
const ENTITY_NAMESPACE: &str = "ghostlight.id.entity.v1";
const CONTROLLER_NAMESPACE: &str = "ghostlight.id.controller.v1";
const AFFORDANCE_NAMESPACE: &str = "ghostlight.id.affordance.v1";

const PLACE: RefKind = RefKind::Entity(EntityKind::Place);

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

/// The one resolution owner for every admission lane. `admits_human` is true
/// only where a human principal may still join the draft-approver set, which is
/// genesis alone; the caller derives it from the empty-state predicate.
pub(super) fn resolve_declarations(
    state_subjects: &BTreeMap<SubjectId, SubjectState>,
    state_entities: &BTreeMap<EntityId, EntityRecord>,
    world_id: WorldId,
    command_id: CommandId,
    patch: &WorldPatch,
    admits_human: bool,
) -> Result<ResolvedPatch, Vec<Mismatch>> {
    let mut mismatches = Vec::new();
    let mut index: BTreeMap<DraftHandle, RefKind> = BTreeMap::new();

    for (position, declaration) in patch.declarations.iter().enumerate() {
        let (handle, label, kind) = match declaration {
            Declaration::Subject(subject) => (
                &subject.handle,
                &subject.label,
                RefKind::Subject(subject.kind),
            ),
            Declaration::Entity(entity) => {
                (&entity.handle, &entity.label, RefKind::Entity(entity.kind))
            }
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

    for declaration in &patch.declarations {
        let Declaration::Subject(subject) = declaration else {
            continue;
        };
        let Some(reference) = &subject.authority_scope else {
            continue;
        };
        match reference {
            Ref::Draft(named) => match index.get(named) {
                None => mismatches.push(Mismatch::UnresolvedDraft {
                    handle: named.clone(),
                    expected: PLACE,
                }),
                Some(kind) if *kind != PLACE => mismatches.push(Mismatch::WrongKind {
                    handle: named.clone(),
                    expected: PLACE,
                    actual: *kind,
                }),
                Some(_) => {}
            },
            Ref::Existing(entity_id) => match state_entities.get(entity_id) {
                None => mismatches.push(Mismatch::UnknownCanonical {
                    handle: subject.handle.clone(),
                    expected: PLACE,
                }),
                Some(record) if record.kind != EntityKind::Place => {
                    mismatches.push(Mismatch::WrongKind {
                        handle: subject.handle.clone(),
                        expected: PLACE,
                        actual: RefKind::Entity(record.kind),
                    });
                }
                Some(_) => {}
            },
        }
    }

    if patch.declarations.is_empty() {
        mismatches.push(Mismatch::NoCanonicalChange);
    }

    if !mismatches.is_empty() {
        mismatches.sort();
        return Err(mismatches);
    }

    // No canonical ID exists above this line. Allocation starts only now.
    let mut collisions = Vec::new();
    let mut entities = Vec::new();
    let mut allocated: BTreeMap<DraftHandle, EntityId> = BTreeMap::new();
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
        if state_entities.contains_key(&entity_id) {
            collisions.push(Mismatch::CanonicalCollision {
                handle: entity.handle.clone(),
            });
        }
        allocated.insert(entity.handle.clone(), entity_id);
        entities.push(ResolvedEntity {
            handle: entity.handle.clone(),
            entity_id,
            entity: EntityRecord {
                label: entity.label.clone(),
                kind: entity.kind,
            },
        });
    }

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
        if state_subjects.contains_key(&subject_id) {
            collisions.push(Mismatch::CanonicalCollision {
                handle: input.handle.clone(),
            });
        }
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
        let authority_scope = match &input.authority_scope {
            None => None,
            Some(Ref::Existing(entity_id)) => Some(*entity_id),
            Some(Ref::Draft(named)) => Some(
                *allocated
                    .get(named)
                    .expect("a draft place reference resolved above"),
            ),
        };
        subjects.push(ResolvedSubject {
            handle: input.handle.clone(),
            subject_id,
            subject: SubjectState {
                label: input.label.clone(),
                kind: input.kind,
                authority_scope,
            },
            controller,
            affordances,
        });
    }

    if !collisions.is_empty() {
        collisions.sort();
        return Err(collisions);
    }

    Ok(ResolvedPatch {
        subjects,
        entities,
        evidence: patch.evidence.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::tests::{
        activate, auth_principal, command, creation, owner, player, submit_owner,
    };
    use crate::world::{
        CallerId, CommandBody, CommandId, KernelError, SubmitReceipt, WorldKernel, WorldPhase,
    };

    fn entity(handle: &str, label: &str, kind: EntityKind) -> Declaration {
        Declaration::Entity(EntityDeclaration {
            handle: DraftHandle::new(handle),
            label: label.into(),
            kind,
        })
    }

    fn institution(handle: &str, label: &str, scope: Option<Ref<EntityId>>) -> Declaration {
        Declaration::Subject(SubjectDeclaration {
            handle: DraftHandle::new(handle),
            label: label.into(),
            kind: SubjectKind::Institution,
            controller: NewController::OperationalAgent,
            affordances: BTreeSet::from([AffordanceKind::Speak]),
            authority_scope: scope,
        })
    }

    fn patch_of(declarations: Vec<Declaration>) -> WorldPatch {
        WorldPatch {
            declarations,
            operations: Vec::new(),
            evidence: Vec::new(),
        }
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

    /// The jurisdiction reference names a place that no declaration and no
    /// partition provides. Nothing commits and nothing is allocated.
    #[test]
    fn run_115_jurisdiction_reference_is_rejected_and_allocates_nothing() {
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
                handle: DraftHandle::new("kharad-rhythm-road"),
                expected: PLACE,
            }]
        );
        assert_eq!(kernel.snapshot().unwrap(), before);
        assert_eq!(kernel.journal.commit_count(), commits_before);
        assert!(kernel.state.entities.is_empty());
        assert_eq!(kernel.state.subjects.len(), before.subjects.len());
    }

    /// The same patch with the place declared beside the institution is one
    /// atomic commit: both records land and the reference resolves to the
    /// place's allocated ID.
    #[test]
    fn the_same_patch_with_the_place_declared_commits_atomically() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let before = kernel.snapshot().unwrap();
        let commits_before = kernel.journal.commit_count();

        let receipt = submit_owner(
            &mut kernel,
            &before,
            admit(patch_of(vec![
                entity("kharad-rhythm-road", "The Rhythm Road", EntityKind::Place),
                institution(
                    "rhythm-authority",
                    "The Rhythm Authority",
                    Some(Ref::Draft(DraftHandle::new("kharad-rhythm-road"))),
                ),
            ])),
        );
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));

        let after = kernel.snapshot().unwrap();
        assert_eq!(after.revision, before.revision + 1);
        assert_eq!(kernel.journal.commit_count(), commits_before + 1);
        assert_eq!(kernel.state.entities.len(), 1);
        let (place_id, record) = kernel.state.entities.iter().next().unwrap();
        assert_eq!(record.label, "The Rhythm Road");
        assert_eq!(record.kind, EntityKind::Place);
        let admitted = kernel
            .state
            .subjects
            .values()
            .find(|subject| subject.label == "The Rhythm Authority")
            .expect("the institution is admitted");
        assert_eq!(admitted.authority_scope, Some(*place_id));
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
                handle: DraftHandle::new("rhythm-tithe"),
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
                authority_scope: None,
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
                handle: DraftHandle::new("cavity-yard"),
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
                authority_scope: None,
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

    #[test]
    fn admit_patch_in_active_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = draft_world(directory.path());
        let active = activate(&mut kernel);
        let commits_before = kernel.journal.commit_count();

        let error = kernel
            .submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(owner()),
                    admit(patch_of(vec![entity(
                        "rhythm-road",
                        "The Rhythm Road",
                        EntityKind::Place,
                    )])),
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
        assert_eq!(kernel.journal.commit_count(), commits_before);
        assert_eq!(kernel.snapshot().unwrap(), active);
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
            authority_scope: None,
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

        let world_id = before.world_id;
        let command_id = CommandId::new();
        let first = resolve_declarations(
            &BTreeMap::new(),
            &BTreeMap::new(),
            world_id,
            command_id,
            &patch,
            false,
        )
        .unwrap();
        let second = resolve_declarations(
            &BTreeMap::new(),
            &BTreeMap::new(),
            world_id,
            command_id,
            &patch,
            false,
        )
        .unwrap();
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
            derive_id(SUBJECT_NAMESPACE, before.world_id, command_id, &handle, None),
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

        let mut reopened = WorldKernel::open(&path, before.world_id).unwrap();
        assert_eq!(reopened.snapshot().unwrap(), before);
        assert_eq!(reopened.journal.commit_count(), 1);
        assert!(reopened.state.entities.is_empty());
        assert!(!reopened.state.subjects.contains_key(&SubjectId(would_be[0])));
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
        let resolved = resolve_declarations(
            &kernel.state.subjects,
            &kernel.state.entities,
            before.world_id,
            CommandId::new(),
            &patch,
            false,
        )
        .unwrap();
        let honest = super::super::WorldEffect::PatchAdmitted {
            resolved: resolved.clone(),
        };

        // Forged: the jurisdiction names a place no partition holds, and the
        // effect no longer carries the entity that would have created it.
        let mut forged = resolved.clone();
        forged.entities.clear();
        forged.subjects[0].subject.authority_scope = Some(super::super::EntityId::issue());
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
}
