//! The invocation pipeline: catalog lookup, role binding, precondition
//! evaluation, effect-ceiling checking, band selection, and lowering.
//!
//! One entry point, [`exercise`], reached from `reduce`'s `ExerciseDecision` arm
//! and from `apply_effect`'s `DecisionExercised` arm, so a forged effect is
//! re-derived by the same function that produced the honest one. Nothing else
//! draws a band, and the draw goes through the one `digest()` owner.

use super::patch::{
    self, Affordance, Bounds, ComponentOpKind, Cost, Precondition, Quantity, RefKind, Role,
    WorldPatch,
};
use super::{
    AccessKind, AffordanceId, CommandId, DecisionEvent, DecisionInvocation, DecisionOpportunity,
    EdgeId, EntityId, EventId, KernelError, Magnitude, SubjectId, Target, WorldId, WorldState,
    digest,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// The complete failure set of one affordance invocation. Sorted and complete
/// like `Mismatch`, and like it never serialized: it states the affordance
/// contract, which no commit records.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ActionMismatch {
    UnboundRole {
        role: Role,
    },
    UnknownRole {
        role: Role,
    },
    DuplicateRoleBinding {
        role: Role,
    },
    UnknownTarget {
        role: Role,
    },
    TargetKindMismatch {
        role: Role,
        expected: RefKind,
        actual: RefKind,
    },
    ActorNotPresent {
        precondition: usize,
    },
    TargetUnreachable {
        precondition: usize,
    },
    InsufficientHolding {
        precondition: usize,
    },
    SlotNotProposed {
        slot: usize,
    },
    UnknownSlot {
        slot: usize,
    },
    DuplicateSlotProposal {
        slot: usize,
    },
    MagnitudeShapeMismatch {
        slot: usize,
    },
    MagnitudeOverCeiling {
        slot: usize,
    },
    ZeroMagnitude {
        slot: usize,
    },
    SpeechRequired,
    SpeechNotCarried,
    EmptySpeech,
}

/// The band draw's whole preimage. Five fields, none of them from the
/// invocation's payload: the proposer varies the attempt and cannot vary the
/// draw. `affordance` is the kernel-resolved granted entry, checked against the
/// grants before this value is built.
#[derive(Serialize)]
struct BandPreimage {
    world_id: WorldId,
    revision: u64,
    command_id: CommandId,
    affordance: AffordanceId,
    band_count: usize,
}

/// Catalog lookup, binding, admission, draw, lowering, event. Stages 1 through 4
/// accumulate one complete rejection set and return it sorted before anything is
/// drawn or lowered, so nothing past the gate can fail on caller input.
pub(super) fn exercise(
    state: &WorldState,
    command_id: CommandId,
    current: &DecisionOpportunity,
    invocation: &DecisionInvocation,
) -> Result<DecisionEvent, KernelError> {
    let entry = state
        .affordance_catalog
        .get(&invocation.affordance)
        .ok_or_else(|| KernelError::Invariant("granted affordance has no entry".into()))?;
    let actor = current.scope.subject_id;
    let mut rejections = Vec::new();

    let bindings = bind_roles(state, entry, invocation, &mut rejections);
    check_preconditions(state, actor, entry, &bindings, &mut rejections);
    check_proposals(entry, invocation, &mut rejections);

    if !rejections.is_empty() {
        rejections.sort();
        return Err(KernelError::ActionRejected(rejections));
    }

    let band = select_band(state, command_id, invocation.affordance, entry)?;
    let operations = lower(entry, band, invocation, &bindings)?;
    let effects = if operations.is_empty() {
        Vec::new()
    } else {
        // The same resolver, the same per-operation preconditions, the same
        // conservation equation the declaration lane passes.
        patch::resolve_patch(
            state,
            command_id,
            &WorldPatch {
                declarations: Vec::new(),
                operations,
                evidence: Vec::new(),
            },
        )
        .map_err(KernelError::PatchRejected)?
        .operations
    };

    let revision = state
        .revision
        .checked_add(1)
        .ok_or_else(|| KernelError::Serialization("world revision overflow".into()))?;
    Ok(DecisionEvent {
        id: EventId::for_command(command_id),
        revision,
        scope: current.scope,
        controller_id: current.controller_id,
        invocation: invocation.clone(),
        band,
        effects,
    })
}

/// Stage 2. The acting subject is never a role: it is always the scope's own
/// subject. A binding that happens to name the actor is legal and unremarkable.
fn bind_roles(
    state: &WorldState,
    entry: &Affordance,
    invocation: &DecisionInvocation,
    rejections: &mut Vec<ActionMismatch>,
) -> BTreeMap<Role, Target> {
    let declared: BTreeMap<&Role, RefKind> = entry
        .roles
        .iter()
        .map(|spec| (&spec.role, spec.kind))
        .collect();
    let mut bound: BTreeMap<Role, Target> = BTreeMap::new();
    let mut seen: BTreeSet<&Role> = BTreeSet::new();
    for binding in &invocation.bindings {
        if !seen.insert(&binding.role) {
            rejections.push(ActionMismatch::DuplicateRoleBinding {
                role: binding.role.clone(),
            });
            continue;
        }
        let Some(expected) = declared.get(&binding.role).copied() else {
            rejections.push(ActionMismatch::UnknownRole {
                role: binding.role.clone(),
            });
            continue;
        };
        let Some(actual) = live_kind(state, binding.target) else {
            rejections.push(ActionMismatch::UnknownTarget {
                role: binding.role.clone(),
            });
            continue;
        };
        let fits = match (expected, actual) {
            (RefKind::Subject(None), RefKind::Subject(_)) => true,
            (expected, actual) => expected == actual,
        };
        if fits {
            bound.insert(binding.role.clone(), binding.target);
        } else {
            rejections.push(ActionMismatch::TargetKindMismatch {
                role: binding.role.clone(),
                expected,
                actual,
            });
        }
    }
    for spec in &entry.roles {
        if !seen.contains(&spec.role) {
            rejections.push(ActionMismatch::UnboundRole {
                role: spec.role.clone(),
            });
        }
    }
    bound
}

/// What a target actually is in committed state. `None` means the target keys no
/// live partition in its namespace.
fn live_kind(state: &WorldState, target: Target) -> Option<RefKind> {
    match target {
        Target::Subject(subject_id) => state
            .subjects
            .get(&subject_id)
            .map(|subject| RefKind::Subject(Some(subject.kind))),
        Target::Entity(entity_id) => state
            .entities
            .get(&entity_id)
            .map(|record| RefKind::Entity(record.kind)),
        Target::Edge(edge_id) => state
            .edges
            .get(&edge_id)
            .map(|_| RefKind::Edge(patch::EdgeKind::Route)),
    }
}

/// Stage 3, in declaration order, each failure carrying its index so a test can
/// name the exact precondition that refused.
fn check_preconditions(
    state: &WorldState,
    actor: SubjectId,
    entry: &Affordance,
    bindings: &BTreeMap<Role, Target>,
    rejections: &mut Vec<ActionMismatch>,
) {
    for (index, precondition) in entry.preconditions.iter().enumerate() {
        match precondition {
            Precondition::Present { at } => {
                let Some(Target::Entity(place)) = bindings.get(at).copied() else {
                    continue;
                };
                if state.positions.get(&actor).map(|position| position.place) != Some(place) {
                    rejections.push(ActionMismatch::ActorNotPresent {
                        precondition: index,
                    });
                }
            }
            Precondition::Reachable { to, within } => {
                let Some(Target::Entity(place)) = bindings.get(to).copied() else {
                    continue;
                };
                if !reachable(state, actor, place, *within) {
                    rejections.push(ActionMismatch::TargetUnreachable {
                        precondition: index,
                    });
                }
            }
            Precondition::Holds { resource, at_least } => {
                let Some(Target::Entity(entity_id)) = bindings.get(resource).copied() else {
                    continue;
                };
                let held = state
                    .holdings
                    .get(&actor)
                    .and_then(|held| held.get(&entity_id))
                    .map_or(0, |quantity| quantity.0);
                if held < at_least.0 {
                    rejections.push(ActionMismatch::InsufficientHolding {
                        precondition: index,
                    });
                }
            }
        }
    }
}

/// Dijkstra over the live route graph, admitting only open public routes and
/// relaxing in `EdgeId` order, pruning any path whose accumulated cost exceeds
/// `within`. Cost 0 — already standing there — succeeds; `Present` is the strict
/// form. An unplaced actor reaches nothing.
///
/// This reads routes the scope digest does not cover, which is deliberate: the
/// alternative binds a proposal to a transitively large slice of world topology
/// and rejects it whenever anything near the actor commits. Admission runs at
/// commit against live state instead, so a route that closed while the proposal
/// was in flight makes this fail rather than letting a stale path commit.
fn reachable(state: &WorldState, actor: SubjectId, destination: EntityId, within: Cost) -> bool {
    let Some(origin) = state.positions.get(&actor).map(|position| position.place) else {
        return false;
    };
    let mut best: BTreeMap<EntityId, u64> = BTreeMap::from([(origin, 0)]);
    let mut settled: BTreeSet<EntityId> = BTreeSet::new();
    let ceiling = u64::from(within.0);
    loop {
        let Some((place, cost)) = best
            .iter()
            .filter(|(place, _)| !settled.contains(place))
            .min_by_key(|(place, cost)| (**cost, **place))
            .map(|(place, cost)| (*place, *cost))
        else {
            return false;
        };
        if place == destination {
            return true;
        }
        settled.insert(place);
        let outbound: Vec<(EdgeId, EntityId, u64)> = state
            .edges
            .iter()
            .filter(|(_, record)| record.is_open() && record.access() == AccessKind::Public)
            .filter_map(|(edge_id, record)| {
                let (from, to) = record.endpoints();
                let next = if from == place {
                    to
                } else if to == place {
                    from
                } else {
                    return None;
                };
                Some((*edge_id, next, u64::from(record.cost().0)))
            })
            .collect();
        for (_, next, step) in outbound {
            let total = cost + step;
            if total <= ceiling && best.get(&next).is_none_or(|known| total < *known) {
                best.insert(next, total);
            }
        }
    }
}

/// Stage 4. Every slot is checked, not just the ones the selected band names:
/// the proposer cannot see the band, so it commits to a complete attempt, and
/// checking every slot makes the ceiling total.
fn check_proposals(
    entry: &Affordance,
    invocation: &DecisionInvocation,
    rejections: &mut Vec<ActionMismatch>,
) {
    let mut seen: BTreeSet<usize> = BTreeSet::new();
    for proposed in &invocation.proposed {
        if !seen.insert(proposed.slot) {
            rejections.push(ActionMismatch::DuplicateSlotProposal {
                slot: proposed.slot,
            });
            continue;
        }
        let Some(slot) = entry.effect_slots.get(proposed.slot) else {
            rejections.push(ActionMismatch::UnknownSlot {
                slot: proposed.slot,
            });
            continue;
        };
        match (slot.bounds, proposed.magnitude) {
            (Bounds::None, Magnitude::None) => {}
            (Bounds::Quantity(ceiling), Magnitude::Quantity(value)) => {
                if value.0 == 0 {
                    rejections.push(ActionMismatch::ZeroMagnitude {
                        slot: proposed.slot,
                    });
                } else if value.0 > ceiling.0 {
                    rejections.push(ActionMismatch::MagnitudeOverCeiling {
                        slot: proposed.slot,
                    });
                }
            }
            (Bounds::Cost(ceiling), Magnitude::Cost(value)) => {
                if value.0 == 0 {
                    rejections.push(ActionMismatch::ZeroMagnitude {
                        slot: proposed.slot,
                    });
                } else if value.0 > ceiling.0 {
                    rejections.push(ActionMismatch::MagnitudeOverCeiling {
                        slot: proposed.slot,
                    });
                }
            }
            _ => rejections.push(ActionMismatch::MagnitudeShapeMismatch {
                slot: proposed.slot,
            }),
        }
    }
    for slot in 0..entry.effect_slots.len() {
        if !seen.contains(&slot) {
            rejections.push(ActionMismatch::SlotNotProposed { slot });
        }
    }
    // `Utterance` is serde-transparent, so a value that arrived through
    // deserialization rather than through the constructor is re-checked here
    // rather than trusted.
    match (&invocation.speech, entry.carries_speech) {
        (None, true) => rejections.push(ActionMismatch::SpeechRequired),
        (Some(_), false) => rejections.push(ActionMismatch::SpeechNotCarried),
        (Some(speech), true) if !patch::is_canonical_text(speech.as_str()) => {
            rejections.push(ActionMismatch::EmptySpeech);
        }
        _ => {}
    }
}

/// The only entropy in the kernel: one digest, one modulo, one cumulative walk.
/// Modulo rather than rejection sampling — the bias is bounded by
/// `total_weight / 2^64`, and a rejection loop would need its counter in the
/// preimage, which is a second entropy shape for a rounding error nobody can
/// measure.
fn select_band(
    state: &WorldState,
    command_id: CommandId,
    affordance: AffordanceId,
    entry: &Affordance,
) -> Result<usize, KernelError> {
    let draw = digest(&BandPreimage {
        world_id: state.world_id,
        revision: state.revision,
        command_id,
        affordance,
        band_count: entry.outcome_bands.len(),
    })?;
    let head = draw
        .strip_prefix("sha256:")
        .and_then(|hex| hex.get(..16))
        .and_then(|head| u64::from_str_radix(head, 16).ok())
        .ok_or_else(|| KernelError::Invariant("band draw digest is not hex".into()))?;
    let total: u128 = entry
        .outcome_bands
        .iter()
        .map(|band| u128::from(band.weight))
        .sum();
    if total == 0 {
        return Err(KernelError::Invariant(
            "affordance has no outcome band".into(),
        ));
    }
    let mut position = u128::from(head) % total;
    for (index, band) in entry.outcome_bands.iter().enumerate() {
        let weight = u128::from(band.weight);
        if position < weight {
            return Ok(index);
        }
        position -= weight;
    }
    Err(KernelError::Invariant(
        "band weights did not cover the draw".into(),
    ))
}

/// Stage 7. One arm per `ComponentOpKind`, every referent read from the slot's
/// roles resolved through the bindings, every magnitude from the proposal for
/// that slot. An empty band lowers to nothing.
fn lower(
    entry: &Affordance,
    band: usize,
    invocation: &DecisionInvocation,
    bindings: &BTreeMap<Role, Target>,
) -> Result<Vec<patch::ComponentOp>, KernelError> {
    let malformed = || KernelError::Invariant("admitted affordance slot is malformed".into());
    let band = entry
        .outcome_bands
        .get(band)
        .ok_or_else(|| KernelError::Invariant("selected band does not exist".into()))?;
    let mut operations = Vec::new();
    for slot_index in &band.effects {
        let slot = entry.effect_slots.get(*slot_index).ok_or_else(malformed)?;
        let magnitude = invocation
            .proposed
            .iter()
            .find(|proposed| proposed.slot == *slot_index)
            .map(|proposed| proposed.magnitude)
            .ok_or_else(malformed)?;
        let target = |position: usize| -> Result<Target, KernelError> {
            slot.roles
                .get(position)
                .and_then(|role| bindings.get(role))
                .copied()
                .ok_or_else(malformed)
        };
        let subject = |position: usize| -> Result<patch::Ref<SubjectId>, KernelError> {
            match target(position)? {
                Target::Subject(subject_id) => Ok(patch::Ref::Existing(subject_id)),
                _ => Err(malformed()),
            }
        };
        let entity = |position: usize| -> Result<patch::Ref<EntityId>, KernelError> {
            match target(position)? {
                Target::Entity(entity_id) => Ok(patch::Ref::Existing(entity_id)),
                _ => Err(malformed()),
            }
        };
        let edge = |position: usize| -> Result<patch::Ref<EdgeId>, KernelError> {
            match target(position)? {
                Target::Edge(edge_id) => Ok(patch::Ref::Existing(edge_id)),
                _ => Err(malformed()),
            }
        };
        let quantity = || -> Result<Quantity, KernelError> {
            match magnitude {
                Magnitude::Quantity(value) => Ok(value),
                _ => Err(malformed()),
            }
        };
        let dependency = |position: usize| -> Result<patch::DependencyRef, KernelError> {
            Ok(match target(position)? {
                Target::Subject(subject_id) => {
                    patch::DependencyRef::Subject(patch::Ref::Existing(subject_id))
                }
                Target::Entity(entity_id) => {
                    patch::DependencyRef::Resource(patch::Ref::Existing(entity_id))
                }
                Target::Edge(edge_id) => patch::DependencyRef::Route(patch::Ref::Existing(edge_id)),
            })
        };
        operations.push(match slot.op_kind {
            ComponentOpKind::Relocate => patch::ComponentOp::Relocate {
                subject: subject(0)?,
                via: edge(1)?,
            },
            ComponentOpKind::OpenRoute => patch::ComponentOp::OpenRoute { route: edge(0)? },
            ComponentOpKind::CloseRoute => patch::ComponentOp::CloseRoute { route: edge(0)? },
            ComponentOpKind::AlterCost => patch::ComponentOp::AlterCost {
                route: edge(0)?,
                cost: match magnitude {
                    Magnitude::Cost(value) => value,
                    _ => return Err(malformed()),
                },
            },
            ComponentOpKind::Transfer => patch::ComponentOp::Transfer {
                from: subject(0)?,
                to: subject(1)?,
                resource: entity(2)?,
                qty: quantity()?,
            },
            ComponentOpKind::Transform => patch::ComponentOp::Transform {
                holder: subject(0)?,
                from_resource: entity(1)?,
                into_resource: entity(2)?,
                qty: quantity()?,
            },
            ComponentOpKind::Consume => patch::ComponentOp::Consume {
                holder: subject(0)?,
                resource: entity(1)?,
                qty: quantity()?,
            },
            ComponentOpKind::Bind => patch::ComponentOp::Bind {
                subject: subject(0)?,
                target: dependency(1)?,
            },
            ComponentOpKind::Release => patch::ComponentOp::Release {
                subject: subject(0)?,
                target: dependency(1)?,
            },
        });
    }
    Ok(operations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::custody_tests::custody_kernel;
    use crate::world::patch::{ComponentOp, Ref as PatchRef, ResolvedOp};
    use crate::world::tests::{
        OPENING_BALANCE, activate, affordance_named, auth_principal, command, custody_world,
        operations, opportunity_for, player, submit_owner,
    };
    use crate::world::{
        AuthenticatedCaller, CallerId, CommandBody, EntityKind, ProposedEffect, RoleBinding,
        SubmitReceipt, Utterance, WorldEffect, WorldKernel, WorldSnapshot, apply_effect,
    };
    use std::collections::BTreeSet;

    struct Bench {
        kernel: WorldKernel,
        active: WorldSnapshot,
        carry: AffordanceId,
        clerk: SubjectId,
        keeper: SubjectId,
        tithe: EntityId,
        ingot: EntityId,
        yard: EntityId,
        gate: EntityId,
        shutter: EdgeId,
    }

    fn bench(directory: &std::path::Path, title: &str) -> Bench {
        let mut kernel = custody_kernel(directory, title);
        let (topology, custody, active) = custody_world(&mut kernel);
        let carry = affordance_named(&active, "carry");
        Bench {
            kernel,
            active,
            carry,
            clerk: custody.holder,
            keeper: custody.counterparty,
            tithe: custody.tithe,
            ingot: custody.ingot,
            yard: topology.yard,
            gate: topology.gate,
            shutter: topology.shutter,
        }
    }

    fn binding(role: &str, target: Target) -> RoleBinding {
        RoleBinding {
            role: Role(role.into()),
            target,
        }
    }

    impl Bench {
        /// A Carry the clerk proposes. The actor is never a role, so a Carry
        /// that moves the actor's own goods binds `from` to its own id.
        fn carry_of(&self, qty: u64, place: EntityId, resource: EntityId) -> DecisionInvocation {
            DecisionInvocation {
                affordance: self.carry,
                bindings: vec![
                    binding("from", Target::Subject(self.clerk)),
                    binding("recipient", Target::Subject(self.keeper)),
                    binding("place", Target::Entity(place)),
                    binding("resource", Target::Entity(resource)),
                ],
                proposed: vec![ProposedEffect {
                    slot: 0,
                    magnitude: Magnitude::Quantity(Quantity(qty)),
                }],
                speech: None,
            }
        }

        fn carry(&self, qty: u64, place: EntityId) -> DecisionInvocation {
            self.carry_of(qty, place, self.tithe)
        }

        fn clerk_opportunity(&self) -> DecisionOpportunity {
            opportunity_for(&self.active, self.clerk)
        }

        fn try_exercise(
            &self,
            invocation: &DecisionInvocation,
        ) -> Result<DecisionEvent, KernelError> {
            exercise(
                &self.kernel.state,
                CommandId::issue(),
                &self.clerk_opportunity(),
                invocation,
            )
        }

        fn rejections(&self, invocation: &DecisionInvocation) -> Vec<ActionMismatch> {
            match self.try_exercise(invocation) {
                Err(KernelError::ActionRejected(rejected)) => rejected,
                other => panic!("expected an action rejection, got {other:?}"),
            }
        }

        fn commit(&mut self, invocation: DecisionInvocation) -> SubmitReceipt {
            let opportunity = self.clerk_opportunity();
            let caller = CallerId::Controller(opportunity.controller_id);
            self.kernel
                .submit(
                    command(
                        &self.active,
                        CommandId::new(),
                        caller.clone(),
                        CommandBody::ExerciseDecision {
                            opportunity,
                            invocation,
                        },
                    ),
                    &AuthenticatedCaller::fixture(caller),
                )
                .expect("the invocation commits")
        }

        fn held(&self, holder: SubjectId) -> u64 {
            self.kernel
                .state
                .holdings
                .get(&holder)
                .and_then(|held| held.get(&self.tithe))
                .map_or(0, |quantity| quantity.0)
        }

        /// The same Carry attempt against a sibling entry: one dial moved, one
        /// name. The catalog is Draft-only, so a variant is declared rather
        /// than patched into committed state.
        fn variant(&self, invocation: &DecisionInvocation, kind: &str) -> DecisionInvocation {
            DecisionInvocation {
                affordance: affordance_named(&self.active, kind),
                ..invocation.clone()
            }
        }

        fn refresh(&mut self) {
            self.active = self.kernel.snapshot().unwrap();
        }
    }

    #[test]
    fn a_missing_present_precondition_names_itself() {
        let directory = tempfile::tempdir().unwrap();
        let bench = bench(directory.path(), "Present");
        let before = bench.kernel.state.clone();

        // The clerk stands in the yard, so a Carry bound to the gate fails the
        // first precondition and nothing else.
        assert_eq!(
            bench.rejections(&bench.carry(1, bench.gate)),
            vec![ActionMismatch::ActorNotPresent { precondition: 0 }]
        );
        assert_eq!(bench.kernel.state.revision, before.revision);
        assert_eq!(bench.kernel.state.holdings, before.holdings);
        assert_eq!(bench.kernel.state.events, before.events);
    }

    #[test]
    fn spending_more_than_held_fails_the_holds_precondition() {
        let directory = tempfile::tempdir().unwrap();
        let bench = bench(directory.path(), "Holds");
        let greedy = bench.variant(&bench.carry(1, bench.yard), "carry_greedy");
        assert_eq!(
            bench.rejections(&greedy),
            vec![ActionMismatch::InsufficientHolding { precondition: 1 }]
        );

        // Absence is zero, so a resource the clerk holds none of gives the same
        // name at the boundary rather than a second one.
        let ingot = bench.variant(&bench.carry_of(1, bench.yard, bench.ingot), "carry_holds");
        assert_eq!(
            bench.rejections(&ingot),
            vec![ActionMismatch::InsufficientHolding { precondition: 0 }]
        );
    }

    #[test]
    fn a_reachable_precondition_reads_only_open_public_routes() {
        let directory = tempfile::tempdir().unwrap();
        let mut bench = bench(directory.path(), "Reachable");
        // Yard to gate over open public routes is the ramp (12) plus the span
        // (7): the shutter is closed and the toll stair is restricted. A budget
        // of NEAR_REACH covers only the shutter, FAR_REACH covers the two-hop
        // path.
        let attempt = bench.carry(1, bench.gate);
        let near = bench.variant(&attempt, "carry_near");
        let far = bench.variant(&attempt, "carry_far");
        assert_eq!(
            bench.rejections(&near),
            vec![ActionMismatch::TargetUnreachable { precondition: 0 }]
        );
        assert!(bench.try_exercise(&far).is_ok());

        // Opening the one-hop shutter shortens the path, and closing it again
        // restores the two-hop cost: the check reads the live graph rather than
        // anything the proposal was bound against.
        let before = bench.kernel.snapshot().unwrap();
        submit_owner(
            &mut bench.kernel,
            &before,
            operations(vec![ComponentOp::OpenRoute {
                route: PatchRef::Existing(bench.shutter),
            }]),
        );
        bench.refresh();
        assert!(bench.try_exercise(&near).is_ok());

        let before = bench.kernel.snapshot().unwrap();
        submit_owner(
            &mut bench.kernel,
            &before,
            operations(vec![ComponentOp::CloseRoute {
                route: PatchRef::Existing(bench.shutter),
            }]),
        );
        bench.refresh();
        assert_eq!(
            bench.rejections(&near),
            vec![ActionMismatch::TargetUnreachable { precondition: 0 }]
        );
    }

    #[test]
    fn an_unbound_or_miskinded_role_is_rejected_with_the_complete_set() {
        let directory = tempfile::tempdir().unwrap();
        let bench = bench(directory.path(), "Roles");
        let mut invocation = bench.carry(1, bench.yard);
        // Drop `recipient`, add an undeclared role, and bind `resource` to a
        // place: one rejection carrying all three, sorted.
        invocation.bindings = vec![
            binding("from", Target::Subject(bench.clerk)),
            binding("place", Target::Entity(bench.yard)),
            binding("resource", Target::Entity(bench.yard)),
            binding("porter", Target::Subject(bench.keeper)),
        ];
        let mut expected = vec![
            ActionMismatch::UnboundRole {
                role: Role("recipient".into()),
            },
            ActionMismatch::UnknownRole {
                role: Role("porter".into()),
            },
            ActionMismatch::TargetKindMismatch {
                role: Role("resource".into()),
                expected: RefKind::Entity(EntityKind::Resource),
                actual: RefKind::Entity(EntityKind::Place),
            },
        ];
        expected.sort();
        assert_eq!(bench.rejections(&invocation), expected);
    }

    #[test]
    fn a_duplicated_or_unknown_binding_target_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let bench = bench(directory.path(), "Bindings");
        let mut twice = bench.carry(1, bench.yard);
        twice
            .bindings
            .push(binding("resource", Target::Entity(bench.tithe)));
        assert!(
            bench
                .rejections(&twice)
                .contains(&ActionMismatch::DuplicateRoleBinding {
                    role: Role("resource".into())
                })
        );

        let mut unknown = bench.carry(1, bench.yard);
        unknown.bindings[3] = binding("resource", Target::Entity(EntityId::issue()));
        assert!(
            bench
                .rejections(&unknown)
                .contains(&ActionMismatch::UnknownTarget {
                    role: Role("resource".into())
                })
        );
    }

    #[test]
    fn a_magnitude_over_its_slot_ceiling_is_rejected_and_within_it_commits() {
        let directory = tempfile::tempdir().unwrap();
        let mut bench = bench(directory.path(), "Ceiling");
        assert_eq!(
            bench.rejections(&bench.carry(4, bench.yard)),
            vec![ActionMismatch::MagnitudeOverCeiling { slot: 0 }]
        );

        let clerk_before = bench.held(bench.clerk);
        let keeper_before = bench.held(bench.keeper);
        let receipt = bench.commit(bench.carry(3, bench.yard));
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
        assert_eq!(bench.held(bench.clerk), clerk_before - 3);
        assert_eq!(bench.held(bench.keeper), keeper_before + 3);
        assert_eq!(
            bench.held(bench.clerk) + bench.held(bench.keeper),
            clerk_before + keeper_before,
            "the conserved total is unchanged"
        );
    }

    #[test]
    fn a_magnitude_of_the_wrong_shape_or_zero_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let bench = bench(directory.path(), "Shape");
        for magnitude in [Magnitude::Cost(Cost(2)), Magnitude::None] {
            let mut invocation = bench.carry(1, bench.yard);
            invocation.proposed[0].magnitude = magnitude;
            assert_eq!(
                bench.rejections(&invocation),
                vec![ActionMismatch::MagnitudeShapeMismatch { slot: 0 }]
            );
        }
        assert_eq!(
            bench.rejections(&bench.carry(0, bench.yard)),
            vec![ActionMismatch::ZeroMagnitude { slot: 0 }]
        );
    }

    #[test]
    fn every_slot_must_be_proposed_exactly_once() {
        let directory = tempfile::tempdir().unwrap();
        let bench = bench(directory.path(), "Slots");
        let mut missing = bench.carry(1, bench.yard);
        missing.proposed.clear();
        assert_eq!(
            bench.rejections(&missing),
            vec![ActionMismatch::SlotNotProposed { slot: 0 }]
        );

        let mut twice = bench.carry(1, bench.yard);
        twice.proposed.push(twice.proposed[0]);
        assert_eq!(
            bench.rejections(&twice),
            vec![ActionMismatch::DuplicateSlotProposal { slot: 0 }]
        );

        let mut unknown = bench.carry(1, bench.yard);
        unknown.proposed.push(ProposedEffect {
            slot: 7,
            magnitude: Magnitude::None,
        });
        assert_eq!(
            bench.rejections(&unknown),
            vec![ActionMismatch::UnknownSlot { slot: 7 }]
        );
    }

    #[test]
    fn speech_is_required_exactly_when_the_entry_carries_it() {
        let directory = tempfile::tempdir().unwrap();
        let bench = bench(directory.path(), "Speech");
        let mut spoken_carry = bench.carry(1, bench.yard);
        spoken_carry.speech = Some(Utterance::new("Take it.").unwrap());
        assert_eq!(
            bench.rejections(&spoken_carry),
            vec![ActionMismatch::SpeechNotCarried]
        );

        let speak = affordance_named(&bench.active, "speak");
        let mut silent_speak = DecisionInvocation {
            affordance: speak,
            bindings: Vec::new(),
            proposed: Vec::new(),
            speech: None,
        };
        assert_eq!(
            bench.rejections(&silent_speak),
            vec![ActionMismatch::SpeechRequired]
        );

        // `Utterance` is serde-transparent, so a value that arrived without the
        // constructor is re-checked here rather than trusted.
        silent_speak.speech = Some(serde_json::from_str::<Utterance>("\"   \"").unwrap());
        assert_eq!(
            bench.rejections(&silent_speak),
            vec![ActionMismatch::EmptySpeech]
        );
    }

    #[test]
    fn the_same_command_id_always_selects_the_same_band() {
        let directory = tempfile::tempdir().unwrap();
        let bench = bench(directory.path(), "Bands");
        // Three equally weighted bands, so the draw alone decides.
        let opportunity = bench.clerk_opportunity();
        let invocation = bench.variant(&bench.carry(1, bench.yard), "carry_chance");
        let draw = |command_id: CommandId, invocation: &DecisionInvocation| {
            exercise(&bench.kernel.state, command_id, &opportunity, invocation)
                .expect("the invocation is admissible")
                .band
        };

        let command_id = CommandId::issue();
        let fixed = draw(command_id, &invocation);
        for _ in 0..100 {
            assert_eq!(draw(command_id, &invocation), fixed);
        }

        // No caller-supplied value influences the draw: not a binding order,
        // not a magnitude.
        let mut reordered = invocation.clone();
        reordered.bindings.reverse();
        assert_eq!(draw(command_id, &reordered), fixed);
        let mut heavier = invocation.clone();
        heavier.proposed[0].magnitude = Magnitude::Quantity(Quantity(3));
        assert_eq!(draw(command_id, &heavier), fixed);

        // The command id is the only varying term at one revision.
        let mut seen = BTreeSet::new();
        for _ in 0..64 {
            seen.insert(draw(CommandId::issue(), &invocation));
        }
        assert!(
            seen.len() > 1,
            "varying only the command id must reach more than one band"
        );
    }

    #[test]
    fn a_zero_effect_band_commits_nothing_but_the_event() {
        let directory = tempfile::tempdir().unwrap();
        let mut bench = bench(directory.path(), "EmptyBand");
        let idle = bench.variant(&bench.carry(3, bench.yard), "carry_idle");

        let holdings = bench.kernel.state.holdings.clone();
        let positions = bench.kernel.state.positions.clone();
        let edges = bench.kernel.state.edges.clone();
        let revision = bench.kernel.state.revision;

        let event = bench
            .try_exercise(&idle)
            .expect("an empty band is admissible");
        assert_eq!(event.band, 0);
        assert!(event.effects.is_empty());

        let receipt = bench.commit(idle);
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
        assert_eq!(bench.kernel.state.holdings, holdings);
        assert_eq!(bench.kernel.state.positions, positions);
        assert_eq!(bench.kernel.state.edges, edges);
        assert_eq!(bench.kernel.state.events.len(), 1);
        assert_eq!(bench.kernel.state.revision, revision + 1);
    }

    #[test]
    fn a_committed_event_carries_the_exact_lowered_operations() {
        let directory = tempfile::tempdir().unwrap();
        let mut bench = bench(directory.path(), "Lowering");
        let receipt = bench.commit(bench.carry(2, bench.yard));
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
        let event = bench
            .kernel
            .state
            .events
            .last()
            .expect("the committed event");
        assert_eq!(
            event.effects,
            vec![ResolvedOp::Transfer {
                from: bench.clerk,
                to: bench.keeper,
                resource: bench.tithe,
                qty: Quantity(2),
            }]
        );
    }

    #[test]
    fn a_forged_band_or_forged_effect_does_not_apply() {
        let directory = tempfile::tempdir().unwrap();
        let bench = bench(directory.path(), "ForgedAction");
        let opportunity = bench.clerk_opportunity();
        let command_id = CommandId::issue();
        let honest = exercise(
            &bench.kernel.state,
            command_id,
            &opportunity,
            &bench.carry(2, bench.yard),
        )
        .unwrap();
        let caller = CallerId::Controller(opportunity.controller_id);

        for forged in [
            DecisionEvent {
                band: 3,
                ..honest.clone()
            },
            DecisionEvent {
                effects: vec![ResolvedOp::Transfer {
                    from: bench.clerk,
                    to: bench.keeper,
                    resource: bench.tithe,
                    qty: Quantity(OPENING_BALANCE),
                }],
                ..honest.clone()
            },
        ] {
            let mut candidate = bench.kernel.state.clone();
            let error = apply_effect(
                &mut candidate,
                command_id,
                &caller,
                &WorldEffect::DecisionExercised {
                    opportunity: opportunity.clone(),
                    event: forged,
                },
            )
            .unwrap_err();
            assert!(matches!(error, KernelError::Invariant(_)));
            assert_eq!(candidate, bench.kernel.state);
        }

        // The honest event still applies through the same arm.
        let mut candidate = bench.kernel.state.clone();
        apply_effect(
            &mut candidate,
            command_id,
            &caller,
            &WorldEffect::DecisionExercised {
                opportunity,
                event: honest,
            },
        )
        .unwrap();
        assert_ne!(candidate.holdings, bench.kernel.state.holdings);
    }

    #[test]
    fn the_kernel_speak_entry_commits_its_utterance_and_no_operation() {
        let directory = tempfile::tempdir().unwrap();
        let mut kernel = custody_kernel(directory.path(), "SpeakEntry");
        let _topology = activate(&mut kernel);
        let active = kernel.snapshot().unwrap();
        let speak = affordance_named(&active, "speak");
        let human = active
            .subjects
            .iter()
            .find(|subject| subject.human_controller.is_some())
            .expect("the human subject")
            .id;
        let holdings = kernel.state.holdings.clone();
        let receipt = kernel
            .submit(
                command(
                    &active,
                    CommandId::new(),
                    CallerId::Principal(player()),
                    CommandBody::ExerciseDecision {
                        opportunity: opportunity_for(&active, human),
                        invocation: DecisionInvocation {
                            affordance: speak,
                            bindings: Vec::new(),
                            proposed: Vec::new(),
                            speech: Some(Utterance::new("I open the door.").unwrap()),
                        },
                    },
                ),
                &auth_principal(player()),
            )
            .expect("the Speak entry commits");
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
        let event = kernel.state.events.last().unwrap();
        assert_eq!(event.band, 0);
        assert!(event.effects.is_empty());
        assert_eq!(
            event.invocation.speech.as_ref().map(Utterance::as_str),
            Some("I open the door.")
        );
        assert_eq!(kernel.state.holdings, holdings);
    }
}
