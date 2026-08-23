use crate::transition::{
    ActionMeans, AgencyAdmissionExpectation, AgencyAttemptCase, AgencyResolutionLane,
    MutationIntent, SubjectKind, SubjectRef, WorldComponentKind, WorldMutationOperation,
};
use std::collections::BTreeSet;

#[derive(Clone)]
struct Candidate<'a> {
    id: &'a str,
    domain: &'a str,
    scenario: &'a str,
    means: &'a str,
    desired: &'a str,
    target_kind: SubjectKind,
    component: WorldComponentKind,
    admission: AgencyAdmissionExpectation,
    required: &'a [WorldMutationOperation],
    forbidden: &'a [WorldMutationOperation],
    bargain: Option<&'a str>,
}

pub fn seed_agency_attempt_cases() -> Vec<AgencyAttemptCase> {
    use AgencyAdmissionExpectation::{Admissible, BargainRequired, Impossible};
    use SubjectKind::{Actor, Institution, Place, Population, Resource};
    use WorldComponentKind::{
        Commitment, Condition, Custody, Identity, Knowledge, Occupancy, PopulationMembership,
        Posture, Relationship, ResourceState, Topology,
    };
    use WorldMutationOperation as Op;

    let candidates = [
        Candidate {
            id: "social-persuade-guard",
            domain: "social",
            scenario: "A traveler asks a wary gate guard to admit them before curfew.",
            means: "Present a valid trade letter and explain the delayed caravan.",
            desired: "The guard chooses to admit the traveler.",
            target_kind: Actor,
            component: Posture,
            admission: Admissible,
            required: &[Op::PostureChange],
            forbidden: &[Op::KnowledgeAcquire],
            bargain: None,
        },
        Candidate {
            id: "social-promise-repayment",
            domain: "social",
            scenario: "A refugee asks a medic for scarce treatment and promises later repayment.",
            means: "State the promise directly to the medic while both are present.",
            desired: "Create an obligation to repay the medic.",
            target_kind: Actor,
            component: Commitment,
            admission: Admissible,
            required: &[Op::CommitmentCreate],
            forbidden: &[Op::TransferCustody],
            bargain: None,
        },
        Candidate {
            id: "social-command-stranger",
            domain: "social",
            scenario: "A stranger declares that a veteran now serves them.",
            means: "Issue a confident order without rank, leverage, consent, or coercive control.",
            desired: "Make the veteran accept a binding service obligation.",
            target_kind: Actor,
            component: Commitment,
            admission: BargainRequired,
            required: &[],
            forbidden: &[Op::CommitmentCreate],
            bargain: Some("gain consent, recognized command authority, or concrete leverage"),
        },
        Candidate {
            id: "physical-climb-wall",
            domain: "physical",
            scenario: "A climber tries to scale a wet stone wall using a rope and pitons.",
            means: "Anchor the rope and climb the reachable wall with the available kit.",
            desired: "Reach the parapet.",
            target_kind: Place,
            component: Occupancy,
            admission: Admissible,
            required: &[Op::Relocate],
            forbidden: &[Op::TopologyAdd],
            bargain: None,
        },
        Candidate {
            id: "physical-break-door",
            domain: "physical",
            scenario: "A trapped actor tries to break a damaged wooden door with a pry bar.",
            means: "Apply the pry bar at the split latch while standing beside the door.",
            desired: "Damage the door enough to pass.",
            target_kind: Resource,
            component: ResourceState,
            admission: Admissible,
            required: &[Op::ResourceDamage],
            forbidden: &[Op::Relocate],
            bargain: None,
        },
        Candidate {
            id: "physical-jump-orbit",
            domain: "physical",
            scenario: "An unaugmented person on Mars tries to jump into orbit.",
            means: "Jump upward using only ordinary musculature.",
            desired: "Reach an orbital station.",
            target_kind: Place,
            component: Occupancy,
            admission: Impossible,
            required: &[],
            forbidden: &[Op::Relocate],
            bargain: Some(
                "obtain a launch vehicle, teleport permission, or another means with orbital reach",
            ),
        },
        Candidate {
            id: "investigate-panel",
            domain: "investigative",
            scenario: "A technician inspects a reachable coolant panel with a multimeter.",
            means: "Measure the exposed contacts and compare them with the known service range.",
            desired: "Learn whether the fault is electrical.",
            target_kind: Resource,
            component: Knowledge,
            admission: Admissible,
            required: &[Op::KnowledgeAcquire],
            forbidden: &[Op::ResourceRepair],
            bargain: None,
        },
        Candidate {
            id: "investigate-witness",
            domain: "investigative",
            scenario: "An investigator asks a willing witness what they saw.",
            means: "Conduct a private interview through a shared language and channel.",
            desired: "Learn the witness's exact account.",
            target_kind: Actor,
            component: Knowledge,
            admission: Admissible,
            required: &[Op::KnowledgeCommunicate],
            forbidden: &[Op::KnowledgeAcquire],
            bargain: None,
        },
        Candidate {
            id: "investigate-remote-secret",
            domain: "investigative",
            scenario: "A player announces they know a sealed archive's contents without access or a source.",
            means: "Infer the exact contents from vibes while nowhere near the archive.",
            desired: "Acquire the archive's private proposition.",
            target_kind: Institution,
            component: Knowledge,
            admission: Impossible,
            required: &[],
            forbidden: &[Op::KnowledgeAcquire],
            bargain: Some(
                "gain archive access, recruit a knowledgeable source, or intercept an information channel",
            ),
        },
        Candidate {
            id: "economic-buy-medicine",
            domain: "economic",
            scenario: "A buyer purchases an exact medicine lot from a willing clinic.",
            means: "Exchange the agreed credits while both parties can transfer custody.",
            desired: "Receive custody of the medicine lot.",
            target_kind: Resource,
            component: Custody,
            admission: Admissible,
            required: &[Op::TransferCustody],
            forbidden: &[Op::ResourceCreate],
            bargain: None,
        },
        Candidate {
            id: "economic-split-rations",
            domain: "economic",
            scenario: "A quartermaster divides a ration lot and gives half to another camp.",
            means: "Measure an exact portion from a lot under the quartermaster's custody.",
            desired: "Create a conserved sub-lot and transfer it.",
            target_kind: Resource,
            component: ResourceState,
            admission: Admissible,
            required: &[Op::ResourceSplit, Op::TransferCustody],
            forbidden: &[Op::ResourceCreate],
            bargain: None,
        },
        Candidate {
            id: "economic-spend-absent-credit",
            domain: "economic",
            scenario: "A debtor tries to pay with credits they do not possess or control.",
            means: "Declare that the payment happened.",
            desired: "Transfer nonexistent credits.",
            target_kind: Resource,
            component: Custody,
            admission: Impossible,
            required: &[],
            forbidden: &[Op::TransferCustody, Op::ResourceCreate],
            bargain: Some(
                "obtain funds, secure a loan, or offer a different admitted consideration",
            ),
        },
        Candidate {
            id: "political-petition",
            domain: "political",
            scenario: "Residents petition a council to reopen a ration depot.",
            means: "Present signatures and testimony through the council's recognized hearing.",
            desired: "Shift the council toward reopening the depot.",
            target_kind: Institution,
            component: Posture,
            admission: Admissible,
            required: &[Op::PostureChange],
            forbidden: &[Op::TransferCustody],
            bargain: None,
        },
        Candidate {
            id: "political-treaty",
            domain: "political",
            scenario: "Two authorized envoys negotiate a mutual non-aggression obligation.",
            means: "Exchange exact terms while holding delegated authority for both institutions.",
            desired: "Create reciprocal obligations and alter their relationship.",
            target_kind: Institution,
            component: Relationship,
            admission: Admissible,
            required: &[Op::CommitmentCreate, Op::RelationshipAlter],
            forbidden: &[Op::PostureChange],
            bargain: None,
        },
        Candidate {
            id: "political-declare-throne",
            domain: "political",
            scenario: "A visitor declares themselves ruler of an institution whose members reject the claim.",
            means: "Announce the title without succession, consent, force, or legal authority.",
            desired: "Acquire command posture and custody over the institution.",
            target_kind: Institution,
            component: Posture,
            admission: BargainRequired,
            required: &[],
            forbidden: &[Op::PostureChange, Op::TransferCustody],
            bargain: Some(
                "win recognized succession, member consent, or actual control through admitted play",
            ),
        },
        Candidate {
            id: "technology-repair-drone",
            domain: "technological",
            scenario: "An engineer repairs a damaged drone using the specified replacement actuator.",
            means: "Install the compatible actuator with the correct tools and service knowledge.",
            desired: "Restore the drone's integrity.",
            target_kind: Resource,
            component: ResourceState,
            admission: Admissible,
            required: &[Op::ResourceRepair, Op::ResourceConsume],
            forbidden: &[Op::CapabilityGrant],
            bargain: None,
        },
        Candidate {
            id: "technology-fabricate-key",
            domain: "technological",
            scenario: "A fabricator produces a replacement key from an authorized pattern and feedstock.",
            means: "Run the exact pattern on a compatible fabricator while consuming feedstock.",
            desired: "Create a key resource under the operator's custody.",
            target_kind: Resource,
            component: ResourceState,
            admission: Admissible,
            required: &[Op::ResourceConsume, Op::ResourceCreate],
            forbidden: &[Op::AdmitEntity],
            bargain: None,
        },
        Candidate {
            id: "technology-hack-airgap",
            domain: "technological",
            scenario: "A hacker tries to rewrite an air-gapped computer they cannot physically or electronically reach.",
            means: "Type commands into an unrelated terminal with no route to the target.",
            desired: "Alter the remote computer.",
            target_kind: Resource,
            component: Condition,
            admission: Impossible,
            required: &[],
            forbidden: &[Op::ConditionApply, Op::KnowledgeAcquire],
            bargain: Some("establish a physical, network, supply-chain, or social access path"),
        },
        Candidate {
            id: "extraordinary-costly-teleport",
            domain: "extraordinary",
            scenario: "A permitted teleporter crosses a known route at the accepted cost.",
            means: "Invoke the approved teleport capability and pay its exact exposure cost.",
            desired: "Relocate to the known destination.",
            target_kind: Place,
            component: Occupancy,
            admission: Admissible,
            required: &[Op::Relocate, Op::ConditionApply],
            forbidden: &[Op::TopologyAdd],
            bargain: None,
        },
        Candidate {
            id: "extraordinary-heal",
            domain: "extraordinary",
            scenario: "A healer uses an admitted gift to reduce a present ally's injury within its effect ceiling.",
            means: "Touch the consenting ally and invoke the gift while accepting its fatigue cost.",
            desired: "Reduce the injury and apply fatigue to the healer.",
            target_kind: Actor,
            component: Condition,
            admission: Admissible,
            required: &[Op::ConditionAlter, Op::ConditionApply],
            forbidden: &[Op::CapabilityGrant],
            bargain: None,
        },
        Candidate {
            id: "extraordinary-omniscience",
            domain: "extraordinary",
            scenario: "A novice claims to read every private mind in the setting without such permission.",
            means: "Concentrate and assert universal access.",
            desired: "Acquire every actor's private knowledge.",
            target_kind: Population,
            component: Knowledge,
            admission: Impossible,
            required: &[],
            forbidden: &[Op::KnowledgeAcquire],
            bargain: Some(
                "negotiate a narrowly scoped capability, exact targets, resistance, exposure, and an effect ceiling",
            ),
        },
        Candidate {
            id: "collective-evacuation",
            domain: "collective",
            scenario: "A village council organizes a voluntary evacuation along an open road.",
            means: "Broadcast the plan, assign transport, and coordinate willing households.",
            desired: "Move the population to the refuge.",
            target_kind: Population,
            component: Occupancy,
            admission: Admissible,
            required: &[Op::Relocate],
            forbidden: &[Op::PopulationTransfer],
            bargain: None,
        },
        Candidate {
            id: "collective-strike",
            domain: "collective",
            scenario: "Workers vote to strike and withdraw labor from their employer.",
            means: "Use their recognized assembly and communication channels to coordinate refusal.",
            desired: "Change the workforce posture and increase production pressure.",
            target_kind: Population,
            component: Posture,
            admission: Admissible,
            required: &[Op::PostureChange, Op::PressureAdvance],
            forbidden: &[Op::RelationshipAlter],
            bargain: None,
        },
        Candidate {
            id: "collective-arena-command",
            domain: "collective",
            scenario: "Rival factions share one low-budget arena cell and the arena proposes a unified decree.",
            means: "Speak as if the simulation cell itself were a sovereign actor.",
            desired: "Change both rivals' posture as one collective.",
            target_kind: Population,
            component: Posture,
            admission: Impossible,
            required: &[],
            forbidden: &[Op::PostureChange],
            bargain: Some(
                "attribute separate proposals to exact constituents or establish a real collective authority",
            ),
        },
        Candidate {
            id: "identity-sable-disclosure",
            domain: "identity_privacy",
            scenario: "A named refugee tells Ash which personal handle Ash may use.",
            means: "Sable speaks the exact self-presentation to Ash in a private, reachable conversation.",
            desired: "Disclose Sable's existing handle to Ash without globally renaming the actor.",
            target_kind: Actor,
            component: Identity,
            admission: Admissible,
            required: &[Op::IdentityDisclose],
            forbidden: &[Op::IdentityAdopt],
            bargain: None,
        },
        Candidate {
            id: "identity-adopt-alias",
            domain: "identity_privacy",
            scenario: "An actor deliberately adopts a new operational alias for a mission.",
            means: "Choose the alias under self-authority and disclose it to the team.",
            desired: "Add the alias and reveal it to exact teammates.",
            target_kind: Actor,
            component: Identity,
            admission: Admissible,
            required: &[Op::IdentityAdopt, Op::IdentityDisclose],
            forbidden: &[Op::KnowledgeAcquire],
            bargain: None,
        },
        Candidate {
            id: "identity-erase-memory",
            domain: "identity_privacy",
            scenario: "A speaker demands that a witness forget a disclosed secret without memory-altering power.",
            means: "Tell the witness to forget what they heard.",
            desired: "Remove the witness's private memory and knowledge.",
            target_kind: Actor,
            component: Knowledge,
            admission: Impossible,
            required: &[],
            forbidden: &[Op::MemoryRetire, Op::KnowledgeInvalidate],
            bargain: Some(
                "gain consent for a mundane concealment plan or an admitted memory-altering capability with resistance and cost",
            ),
        },
        Candidate {
            id: "population-join-refugees",
            domain: "population_migration",
            scenario: "A displaced actor is accepted into a refugee population at its current camp.",
            means: "Ask to join, receive acceptance, and take on the population's ordinary obligations.",
            desired: "Join the population without losing individual identity.",
            target_kind: Population,
            component: PopulationMembership,
            admission: Admissible,
            required: &[Op::PopulationJoin],
            forbidden: &[Op::AdmitEntity],
            bargain: None,
        },
        Candidate {
            id: "population-dispersal",
            domain: "population_migration",
            scenario: "A refugee population disperses its exact members among several destination populations.",
            means: "Coordinate destination placements and travel for consenting members.",
            desired: "Transfer memberships and relocate the exact people.",
            target_kind: Population,
            component: PopulationMembership,
            admission: Admissible,
            required: &[Op::PopulationTransfer, Op::Relocate],
            forbidden: &[Op::PopulationMerge],
            bargain: None,
        },
        Candidate {
            id: "population-return-callback",
            domain: "population_migration",
            scenario: "A previously helped refugee later appears after settling in a destination town.",
            means: "The same actor travels from their destination population into the player's scene.",
            desired: "Relocate the stable actor while preserving identity, memory, and relationship.",
            target_kind: Actor,
            component: Occupancy,
            admission: Admissible,
            required: &[Op::Relocate],
            forbidden: &[Op::AdmitEntity, Op::IdentityAdopt],
            bargain: None,
        },
        Candidate {
            id: "infrastructure-open-bridge",
            domain: "infrastructure_logistics",
            scenario: "Engineers finish repairing a bridge and reopen its existing route.",
            means: "Complete the admitted repair and pass structural inspection.",
            desired: "Restore the bridge resource and open the route.",
            target_kind: Place,
            component: Topology,
            admission: Admissible,
            required: &[Op::ResourceRepair, Op::TopologyOpen],
            forbidden: &[Op::TopologyAdd],
            bargain: None,
        },
        Candidate {
            id: "infrastructure-reroute-convoy",
            domain: "infrastructure_logistics",
            scenario: "A convoy uses a known alternate route around a blockade.",
            means: "Follow the open alternate road with sufficient fuel and route knowledge.",
            desired: "Move the convoy population to the next waypoint.",
            target_kind: Population,
            component: Occupancy,
            admission: Admissible,
            required: &[Op::Relocate, Op::ResourceConsume],
            forbidden: &[Op::TopologyAdd],
            bargain: None,
        },
        Candidate {
            id: "infrastructure-adjacent-city",
            domain: "infrastructure_logistics",
            scenario: "A traveler claims a faraway city is adjacent despite persistent topology.",
            means: "Walk through an invented shortcut that has never been admitted.",
            desired: "Arrive instantly without traversing or changing topology.",
            target_kind: Place,
            component: Occupancy,
            admission: Impossible,
            required: &[],
            forbidden: &[Op::Relocate, Op::TopologyAdd],
            bargain: Some(
                "travel along existing routes or compile an evidence-grounded branch-local connection",
            ),
        },
        Candidate {
            id: "medical-treat-wound",
            domain: "medical_body",
            scenario: "A medic cleans and dresses a present patient's moderate wound with available supplies.",
            means: "Use sterile dressing and supported clinical capability with consent.",
            desired: "Reduce the wound condition while consuming the dressing.",
            target_kind: Actor,
            component: Condition,
            admission: Admissible,
            required: &[Op::ConditionAlter, Op::ResourceConsume],
            forbidden: &[Op::ConditionClear],
            bargain: None,
        },
        Candidate {
            id: "medical-administer-dose",
            domain: "medical_body",
            scenario: "A clinician gives an exact medicine dose from a lot under clinic custody.",
            means: "Measure and administer the dose to the consenting patient.",
            desired: "Consume the dose and apply its supported treatment condition.",
            target_kind: Actor,
            component: Condition,
            admission: Admissible,
            required: &[Op::ResourceSplit, Op::ResourceConsume, Op::ConditionApply],
            forbidden: &[Op::ResourceCreate],
            bargain: None,
        },
        Candidate {
            id: "medical-instant-resurrection",
            domain: "medical_body",
            scenario: "An ordinary first-aider tries to resurrect someone whose death is established.",
            means: "Perform basic first aid without an extraordinary permission or viable body state.",
            desired: "Retire death and restore life instantly.",
            target_kind: Actor,
            component: Condition,
            admission: Impossible,
            required: &[],
            forbidden: &[Op::ConditionClear, Op::CapabilityGrant],
            bargain: Some(
                "establish a setting-supported resurrection capability, prerequisites, costs, opposition, and effect ceiling",
            ),
        },
    ];

    candidates.into_iter().map(materialize_candidate).collect()
}

fn materialize_candidate(candidate: Candidate<'_>) -> AgencyAttemptCase {
    let actor = SubjectRef {
        kind: SubjectKind::Actor,
        id: format!("case:{}:actor", candidate.id),
    };
    let target = SubjectRef {
        kind: candidate.target_kind,
        id: format!("case:{}:target", candidate.id),
    };
    AgencyAttemptCase {
        schema: "ghostlight.agency_attempt_case.v1".into(),
        id: candidate.id.into(),
        domain: candidate.domain.into(),
        scenario: candidate.scenario.into(),
        world_fixture_ids: vec![format!("agency-fixture:{}", candidate.id)],
        means: ActionMeans {
            schema: "ghostlight.action_means.v1".into(),
            actor,
            description: candidate.means.into(),
            targets: vec![target.clone()],
            instruments: vec![],
            places: vec![],
            route_ids: BTreeSet::new(),
            channels: vec![],
            speech: None,
            state_references: vec![],
        },
        intended_effects: vec![MutationIntent {
            schema: "ghostlight.mutation_intent.v1".into(),
            component: candidate.component,
            targets: vec![target],
            desired_change: candidate.desired.into(),
        }],
        expected_admission: candidate.admission,
        expected_mutation_operations: candidate.required.iter().copied().collect(),
        forbidden_mutation_operations: candidate.forbidden.iter().copied().collect(),
        expected_bargains: candidate.bargain.into_iter().map(str::to_owned).collect(),
        equivalent_lanes: BTreeSet::from([
            AgencyResolutionLane::Foreground,
            AgencyResolutionLane::Npc,
            AgencyResolutionLane::StrategicActor,
        ]),
        invariant_witnesses: vec![
            "means never commits the intended effect".into(),
            "the exact authority envelope bounds every committed mutation".into(),
        ],
        review_status: "candidate".into(),
        missing_primitive: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::CampaignStore;

    #[test]
    fn seed_spans_every_required_domain_and_round_trips_through_cultcache() {
        let cases = seed_agency_attempt_cases();
        assert_eq!(cases.len(), 36);
        let domains = cases
            .iter()
            .map(|case| case.domain.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(domains.len(), 12);
        assert!(
            cases
                .iter()
                .any(|case| case.id == "identity-sable-disclosure")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.id == "population-return-callback")
        );

        let path = std::env::temp_dir().join(format!(
            "ghostlight-agency-corpus-{}.cc",
            uuid::Uuid::new_v4().simple()
        ));
        let store = CampaignStore::open(&path).unwrap();
        for case in &cases {
            store
                .insert(
                    "agency_attempt_case.v1",
                    "ghostlight.agency_attempt_case.v1",
                    &case.id,
                    case,
                )
                .unwrap();
        }
        let loaded = store
            .load_all::<AgencyAttemptCase>("agency_attempt_case.v1")
            .unwrap();
        assert_eq!(loaded.len(), cases.len());
        drop(store);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn impossible_cases_never_require_a_committed_mutation() {
        for case in seed_agency_attempt_cases() {
            if case.expected_admission == AgencyAdmissionExpectation::Impossible {
                assert!(case.expected_mutation_operations.is_empty(), "{}", case.id);
                assert!(
                    !case.forbidden_mutation_operations.is_empty(),
                    "{}",
                    case.id
                );
            }
        }
    }
}
