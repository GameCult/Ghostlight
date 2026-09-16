//! Admission proofs for the kernel's seal, from outside the crate.
//!
//! The seal is unforgeable *admission*, not unconstructible values. External
//! code may hold a syntactically valid ID, digest, or `DecisionOpportunity` —
//! a consumer's own payloads carry these types over the wire, so they
//! deserialize — and the kernel must refuse every one it did not itself issue.
//! An integration test compiles as its own crate against the public API alone,
//! so these tests stand exactly where a consumer stands: every forged value
//! below is minted by `serde` from JSON, which is a path visibility does not
//! gate, and every rejection is asserted on the typed error rather than on a
//! message.
//!
//! Each rejection also asserts that nothing committed: revision, state digest,
//! last commit digest, and the operator log's length are unchanged. The
//! operator log is the widest committed-history surface a consumer can read;
//! the journal itself has no public accessor, so its length is proven only
//! through the digests and that log.

use chrono::{Duration, Utc};
use ghostlight::{
    CommandBody, CommandId, ControllerMode, ControllerPort, CreateWorldIntent, DecisionInvocation,
    DecisionOpportunity, KernelError, MailboxError, PrincipalCommandIntent, Statement,
    SubjectId, SubmitReceipt, VerifiedPrincipalEvidence, WorldMailbox, WorldPhase, WorldSnapshot,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// A live world in Active phase, reached through the same public ingress a
/// consumer uses: create, approve, activate. Genesis grants speech to every
/// subject it declares, so the two model-controlled subjects each carry a live
/// opportunity with one granted affordance.
struct World {
    _directory: tempfile::TempDir,
    _owner: tokio::task::JoinHandle<()>,
    mailbox: WorldMailbox,
}

impl World {
    async fn active() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let (mailbox, owner) = WorldMailbox::open(directory.path().join("world.cc")).unwrap();
        let principal =
            VerifiedPrincipalEvidence::new("external-admission-account", Utc::now() + Duration::hours(1));
        let receipt = mailbox
            .create(
                CreateWorldIntent {
                    id: CommandId::new(),
                    title: "External Admission".into(),
                    brief: String::new(),
                    human_subject_label: "Operator".into(),
                    narrative_persona_label: Some("Persona".into()),
                    operational_agent_label: Some("Operational Agent".into()),
                    targets: BTreeMap::new(),
                    jurisdictions: Vec::new(),
                },
                &principal,
            )
            .await
            .unwrap();
        for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
            let revision = mailbox.snapshot().await.unwrap().revision;
            mailbox
                .submit_principal(
                    PrincipalCommandIntent {
                        id: CommandId::new(),
                        world_id: receipt.world_id,
                        expected_revision: revision,
                        body,
                    },
                    &principal,
                )
                .await
                .unwrap();
        }
        let world = Self {
            _directory: directory,
            _owner: owner,
            mailbox,
        };
        assert_eq!(world.snapshot().await.phase, WorldPhase::Active);
        world
    }

    async fn snapshot(&self) -> WorldSnapshot {
        self.mailbox.snapshot().await.unwrap()
    }

    fn port(&self) -> ControllerPort {
        ControllerPort::new(self.mailbox.clone())
    }

    /// The first opportunity a model controller holds. The human subject's
    /// opportunity is not one a controller port may exercise.
    async fn controller_opportunity(&self) -> DecisionOpportunity {
        self.snapshot()
            .await
            .opportunities
            .into_iter()
            .find(|opportunity| opportunity.controller_mode != ControllerMode::Human)
            .expect("an activated world derives a model controller's opportunity")
    }

    /// Everything a commit would move, read through the public surface.
    async fn committed(&self) -> (u64, String, Option<String>, usize) {
        let snapshot = self.snapshot().await;
        let log = self.mailbox.operator_log().await.unwrap();
        (
            snapshot.revision,
            snapshot.state_digest,
            snapshot.last_commit_digest,
            log.len(),
        )
    }
}

/// Forging is a `serde` round trip: the value goes out as JSON, a field is
/// replaced, and it comes back as the sealed type. No field of
/// `DecisionOpportunity` needs to be nameable for this to work, which is the
/// whole reason the kernel cannot rely on visibility here.
fn forge(opportunity: &DecisionOpportunity, field: &str, value: Value) -> DecisionOpportunity {
    let mut encoded = serde_json::to_value(opportunity).expect("a sealed opportunity serializes");
    encoded[field] = value;
    serde_json::from_value(encoded).expect("an external crate deserializes a sealed opportunity")
}

fn speak(affordance: ghostlight::AffordanceId, text: &str) -> DecisionInvocation {
    DecisionInvocation {
        affordance,
        bindings: Vec::new(),
        proposed: Vec::new(),
        speech: Some(Statement::new(text).unwrap()),
        display: None,
    }
}

/// The control. Without it every rejection below could be an inert harness
/// rather than a refusal: this proves the same port, world, and invocation
/// shape do commit when the opportunity is one the kernel issued.
#[tokio::test]
async fn an_issued_opportunity_commits_through_the_same_port() {
    let world = World::active().await;
    let opportunity = world.controller_opportunity().await;
    let (revision, _, _, events) = world.committed().await;
    let receipt = world
        .port()
        .submit_controller(
            CommandId::new(),
            &opportunity,
            speak(opportunity.affordance_ids[0], "The issued proposal stands."),
        )
        .await
        .expect("an issued opportunity is admitted");
    assert!(matches!(receipt, SubmitReceipt::Applied(_)));
    let after = world.committed().await;
    assert_eq!(after.0, revision + 1);
    assert_eq!(after.3, events + 1);
}

#[tokio::test]
async fn a_forged_scope_digest_is_refused_and_commits_nothing() {
    let world = World::active().await;
    let opportunity = world.controller_opportunity().await;
    let before = world.committed().await;
    let forged = forge(
        &opportunity,
        "scope_digest",
        json!("sha256:forged-outside-the-crate"),
    );
    let result = world
        .port()
        .submit_controller(
            CommandId::new(),
            &forged,
            speak(forged.affordance_ids[0], "Let me in."),
        )
        .await;
    assert!(
        matches!(
            result,
            Err(MailboxError::Kernel(KernelError::ScopeChanged { .. }))
        ),
        "a forged scope digest was not refused: {result:?}"
    );
    assert_eq!(world.committed().await, before);
}

/// The value the crate's own deleted `ScopeDigest::fixture` used to produce.
/// A `ScopeDigest` is a transparent newtype over a string, so deserializing
/// that string is the whole of what the fixture did, and it is available to
/// any external crate whatever the type's visibility is. What it is not is
/// admission.
#[tokio::test]
async fn a_scope_digest_deserialized_from_a_string_commits_nothing() {
    let world = World::active().await;
    let opportunity = world.controller_opportunity().await;
    let before = world.committed().await;
    let forged = forge(&opportunity, "scope_digest", json!("sha256:not-a-scope"));
    let result = world
        .port()
        .submit_controller(
            CommandId::new(),
            &forged,
            speak(forged.affordance_ids[0], "A fixture digest is not a scope."),
        )
        .await;
    assert!(
        matches!(
            result,
            Err(MailboxError::Kernel(KernelError::ScopeChanged { .. }))
        ),
        "a deserialized fixture digest was not refused: {result:?}"
    );
    assert_eq!(world.committed().await, before);
}

/// `ControllerId` is not even a nameable type outside the crate, and forging
/// one still needs no name: the field is a UUID on the wire. The port stamps
/// the caller from the opportunity it is handed, so a forged controller
/// speaks as itself — and the reducer refuses it because the scope's
/// assignment names someone else.
#[tokio::test]
async fn a_forged_controller_id_is_refused_and_commits_nothing() {
    let world = World::active().await;
    let opportunity = world.controller_opportunity().await;
    let before = world.committed().await;
    let forged = forge(
        &opportunity,
        "controller_id",
        json!(uuid::Uuid::new_v4().to_string()),
    );
    let result = world
        .port()
        .submit_controller(
            CommandId::new(),
            &forged,
            speak(forged.affordance_ids[0], "I say I am the controller."),
        )
        .await;
    assert!(
        matches!(
            result,
            Err(MailboxError::Kernel(KernelError::ControllerMismatch))
        ),
        "a forged controller ID was not refused: {result:?}"
    );
    assert_eq!(world.committed().await, before);
}

/// A `SubjectId` minted by deserialization — the kernel's `issue()` is
/// private, and this needs it no more than the wire does. The world derives
/// no opportunity for a subject it never declared.
#[tokio::test]
async fn a_command_naming_a_minted_subject_id_is_refused_and_commits_nothing() {
    let world = World::active().await;
    let opportunity = world.controller_opportunity().await;
    let before = world.committed().await;
    let minted: SubjectId = serde_json::from_value(json!(uuid::Uuid::new_v4().to_string()))
        .expect("an external crate mints a syntactically valid SubjectId");
    let forged = forge(&opportunity, "scope", json!({ "subject_id": minted }));
    let result = world
        .port()
        .submit_controller(
            CommandId::new(),
            &forged,
            speak(forged.affordance_ids[0], "A subject who was never declared."),
        )
        .await;
    assert!(
        matches!(
            result,
            Err(MailboxError::Kernel(KernelError::OpportunityMismatch))
        ),
        "a minted subject ID was not refused: {result:?}"
    );
    assert_eq!(world.committed().await, before);
}

/// The same for an `AffordanceId`: the opportunity here is entirely genuine,
/// so the forgery is only the affordance the invocation names. Admission is
/// checked against what the world granted, not against what the command
/// claims.
#[tokio::test]
async fn a_command_naming_a_minted_affordance_id_is_refused_and_commits_nothing() {
    let world = World::active().await;
    let opportunity = world.controller_opportunity().await;
    let before = world.committed().await;
    let minted: ghostlight::AffordanceId =
        serde_json::from_value(json!(uuid::Uuid::new_v4().to_string()))
            .expect("an external crate mints a syntactically valid AffordanceId");
    assert!(!opportunity.affordance_ids.contains(&minted));
    let result = world
        .port()
        .submit_controller(
            CommandId::new(),
            &opportunity,
            speak(minted, "An affordance nobody granted."),
        )
        .await;
    assert!(
        matches!(
            result,
            Err(MailboxError::Kernel(KernelError::AffordanceDenied))
        ),
        "a minted affordance ID was not refused: {result:?}"
    );
    assert_eq!(world.committed().await, before);
}

/// A whole opportunity built from JSON alone, naming a world, a scope, a
/// controller and an affordance the kernel never issued. This is the shape
/// the seal's wording is about: the value exists, and admission does not.
#[tokio::test]
async fn a_wholly_deserialized_opportunity_is_refused_and_commits_nothing() {
    let world = World::active().await;
    let before = world.committed().await;
    let uuid = || json!(uuid::Uuid::new_v4().to_string());
    let affordance = uuid();
    let forged: DecisionOpportunity = serde_json::from_value(json!({
        "world_id": uuid(),
        "revision": 0,
        "scope_digest": "sha256:not-a-scope",
        "scope": { "subject_id": uuid() },
        "controller_id": uuid(),
        "controller_mode": "operational_agent",
        "affordance_ids": [affordance],
    }))
    .expect("an external crate deserializes a whole opportunity");
    let result = world
        .port()
        .submit_controller(
            CommandId::new(),
            &forged,
            speak(forged.affordance_ids[0], "Wholly invented."),
        )
        .await;
    assert!(
        matches!(
            result,
            Err(MailboxError::Kernel(KernelError::OpportunityMismatch))
        ),
        "a wholly forged opportunity was not refused: {result:?}"
    );
    assert_eq!(world.committed().await, before);
}

/// A forgery indistinguishable in shape from a real digest, because it is a
/// real digest: one the kernel genuinely issued for this scope, gone stale
/// when the world moved under it. Only an exact comparison of the whole
/// digest refuses it; a check of its length or of any prefix would admit it.
/// A stale digest still differs early, so the test also submits the current
/// digest with its last digit changed, which a comparison that skips any
/// suffix would admit.
///
/// Genesis places every subject in one commons, so one controller's speech
/// lands as knowledge on the others, and knowledge is a scope component: the
/// listener's digest moves, the speaker's does not.
#[tokio::test]
async fn a_stale_issued_scope_digest_is_refused_and_commits_nothing() {
    let world = World::active().await;
    let controllers: Vec<_> = world
        .snapshot()
        .await
        .opportunities
        .into_iter()
        .filter(|opportunity| opportunity.controller_mode != ControllerMode::Human)
        .collect();
    let [speaker, stale] = <[DecisionOpportunity; 2]>::try_from(controllers)
        .expect("the world declares two model-controlled subjects");
    world
        .port()
        .submit_controller(
            CommandId::new(),
            &speaker,
            speak(speaker.affordance_ids[0], "Everyone in the commons hears this."),
        )
        .await
        .expect("an issued opportunity is admitted");
    let current = world
        .snapshot()
        .await
        .opportunities
        .into_iter()
        .find(|opportunity| opportunity.scope == stale.scope)
        .expect("the listener still holds an opportunity");
    // The digest is not a public field; it is read the way a consumer reads
    // it, off the wire.
    let digest_of = |opportunity: &DecisionOpportunity| {
        serde_json::to_value(opportunity).unwrap()["scope_digest"]
            .as_str()
            .expect("a scope digest is a string on the wire")
            .to_owned()
    };
    let (stale_digest, current_digest) = (digest_of(&stale), digest_of(&current));
    assert_ne!(
        stale_digest, current_digest,
        "the speech did not move the listener's scope digest"
    );
    assert_eq!(stale_digest.len(), current_digest.len());
    let before = world.committed().await;
    let result = world
        .port()
        .submit_controller(
            CommandId::new(),
            &stale,
            speak(stale.affordance_ids[0], "Against a scope that has moved."),
        )
        .await;
    assert!(
        matches!(
            result,
            Err(MailboxError::Kernel(KernelError::ScopeChanged { .. }))
        ),
        "a stale issued scope digest was not refused: {result:?}"
    );
    assert_eq!(world.committed().await, before);

    // A stale digest differs from the current one almost everywhere, so it
    // cannot tell a whole-digest comparison from one that skips a suffix. The
    // current digest with only its final hex digit changed can.
    let mut near = current_digest.clone();
    let last = near.pop().expect("a digest is not empty");
    near.push(if last == '0' { '1' } else { '0' });
    assert_eq!(near.len(), current_digest.len());
    assert_ne!(near, current_digest);
    let forged = forge(&current, "scope_digest", json!(near));
    let result = world
        .port()
        .submit_controller(
            CommandId::new(),
            &forged,
            speak(forged.affordance_ids[0], "One digit from the real scope."),
        )
        .await;
    assert!(
        matches!(
            result,
            Err(MailboxError::Kernel(KernelError::ScopeChanged { .. }))
        ),
        "a digest one digit from the current scope was not refused: {result:?}"
    );
    assert_eq!(world.committed().await, before);
}
