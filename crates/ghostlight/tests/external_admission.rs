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
    DecisionOpportunity, KernelError, Lens, LensWeights, MailboxError, Mismatch, PrincipalCommandIntent,
    Statement, SubjectId, SubmitReceipt, VerifiedPrincipalEvidence, WorldMailbox, WorldPhase,
    WorldSnapshot,
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
    principal: VerifiedPrincipalEvidence,
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
                    // An external crate has no library fixture; it states its
                    // own weights, as every consumer must.
                    lens_weights: LensWeights::new(BTreeMap::from([(Lens::Patina, 1)])),
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
            principal,
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
/// digest changed in one way each that a weaker comparison would admit: its
/// last digit changed (suffix skipped), one hex letter uppercased (case
/// ignored), and its hex bare or under another label (label ignored).
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
    // cannot tell a whole-digest comparison from one that compares less. Each
    // forgery below is the current digest changed in exactly one way that a
    // weaker comparison would admit; each asserts that premise first.
    let (label, hex) = current_digest
        .split_once(':')
        .expect("an issued digest is algorithm:hex");
    assert_eq!(label, "sha256");
    assert_eq!(hex.len(), 64);
    assert!(hex.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')));

    // Only the final hex digit changed: a comparison that skips a suffix.
    let mut near = current_digest.clone();
    let last = near.pop().expect("a digest is not empty");
    near.push(if last == '0' { '1' } else { '0' });
    assert_eq!(near.len(), current_digest.len());
    assert_ne!(near, current_digest);

    // One hex letter uppercased: a case-insensitive comparison.
    let letter = hex
        .find(|c: char| matches!(c, 'a'..='f'))
        .expect("the issued digest has a hex letter to uppercase");
    let mut upper = hex.to_owned();
    upper.replace_range(letter..=letter, &hex[letter..=letter].to_ascii_uppercase());
    let upper = format!("{label}:{upper}");
    assert_ne!(upper, current_digest);
    assert!(upper.eq_ignore_ascii_case(&current_digest));

    // The same hex without its label, and under another label: a comparison of
    // the hex alone.
    let bare = hex.to_owned();
    let relabeled = format!("blake3:{hex}");
    for forgery in [&bare, &relabeled] {
        assert_ne!(forgery, &current_digest);
        assert_eq!(forgery.rsplit(':').next(), Some(hex));
    }

    for (forgery, what) in [
        (near, "one digit from"),
        (upper, "one letter's case from"),
        (bare, "the unlabeled hex of"),
        (relabeled, "the hex under another label of"),
    ] {
        let forged = forge(&current, "scope_digest", json!(forgery));
        let result = world
            .port()
            .submit_controller(
                CommandId::new(),
                &forged,
                speak(forged.affordance_ids[0], "Nearly the real scope."),
            )
            .await;
        assert!(
            matches!(
                result,
                Err(MailboxError::Kernel(KernelError::ScopeChanged { .. }))
            ),
            "a digest {what} the current scope was not refused: {result:?}"
        );
        assert_eq!(world.committed().await, before, "{what}");
    }
}

/// Every stock lens named, with real weights: the shape a consumer's weight
/// control produces, so a refusal below is about the caller and not the value.
fn real_weights() -> LensWeights {
    LensWeights::new(BTreeMap::from([
        (Lens::Patina, 3),
        (Lens::Charter, 1),
        (Lens::Ledger, 0),
        (Lens::Hearth, 2),
        (Lens::Tangle, 1),
        (Lens::Veil, 1),
        (Lens::Ember, 1),
        (Lens::Numen, 1),
    ]))
}

/// A second verified principal holds a live session and valid weights and is
/// not the owner. The kernel refuses it and nothing commits; the same body
/// from the owner then commits, so the refusal is not an inert harness.
#[tokio::test]
async fn set_lens_weights_from_a_non_owner_is_refused_and_commits_nothing() {
    let world = World::active().await;
    let snapshot = world.snapshot().await;
    let stranger =
        VerifiedPrincipalEvidence::new("external-admission-stranger", Utc::now() + Duration::hours(1));
    let before = world.committed().await;
    let result = world
        .mailbox
        .submit_principal(
            PrincipalCommandIntent {
                id: CommandId::new(),
                world_id: snapshot.world_id,
                expected_revision: snapshot.revision,
                body: CommandBody::SetLensWeights {
                    weights: real_weights(),
                },
            },
            &stranger,
        )
        .await;
    assert!(
        matches!(
            result,
            Err(MailboxError::Kernel(KernelError::Unauthorized))
        ),
        "a non-owner replaced the lens weights: {result:?}"
    );
    assert_eq!(world.committed().await, before);
    assert_ne!(world.snapshot().await.lens_weights, real_weights());

    let receipt = world
        .mailbox
        .submit_principal(
            PrincipalCommandIntent {
                id: CommandId::new(),
                world_id: snapshot.world_id,
                expected_revision: snapshot.revision,
                body: CommandBody::SetLensWeights {
                    weights: real_weights(),
                },
            },
            &world.principal,
        )
        .await
        .expect("the owner replaces the lens weights");
    assert!(matches!(receipt, SubmitReceipt::Applied(_)));
    let after = world.snapshot().await;
    assert_eq!(after.revision, before.0 + 1);
    assert_eq!(after.lens_weights, real_weights());
}

/// The owner resubmitting the world's current weights changes nothing
/// canonical, so the kernel refuses it as `NoCanonicalChange` and nothing
/// commits. Admission runs first: the same identical set from a non-owner is
/// `Unauthorized`. A different set from the owner then commits, so neither
/// refusal is an inert harness.
#[tokio::test]
async fn identical_lens_weights_from_the_owner_commit_nothing() {
    let world = World::active().await;
    let current = world.snapshot().await.lens_weights;
    let before = world.committed().await;
    let intent = |snapshot: &WorldSnapshot, weights: LensWeights| PrincipalCommandIntent {
        id: CommandId::new(),
        world_id: snapshot.world_id,
        expected_revision: snapshot.revision,
        body: CommandBody::SetLensWeights { weights },
    };
    let snapshot = world.snapshot().await;

    let result = world
        .mailbox
        .submit_principal(intent(&snapshot, current.clone()), &world.principal)
        .await;
    let Err(MailboxError::Kernel(KernelError::PatchRejected(mismatches))) = &result else {
        panic!("the owner's identical lens weights were not refused as no change: {result:?}");
    };
    assert_eq!(mismatches, &vec![Mismatch::NoCanonicalChange]);
    assert_eq!(world.committed().await, before);

    let stranger =
        VerifiedPrincipalEvidence::new("external-admission-stranger", Utc::now() + Duration::hours(1));
    let result = world
        .mailbox
        .submit_principal(intent(&snapshot, current.clone()), &stranger)
        .await;
    assert!(
        matches!(
            result,
            Err(MailboxError::Kernel(KernelError::Unauthorized))
        ),
        "a non-owner's identical lens weights were not refused as unauthorized: {result:?}"
    );
    assert_eq!(world.committed().await, before);

    assert_ne!(current, real_weights());
    let receipt = world
        .mailbox
        .submit_principal(intent(&snapshot, real_weights()), &world.principal)
        .await
        .expect("the owner replaces the lens weights with a different set");
    assert!(matches!(receipt, SubmitReceipt::Applied(_)));
    let after = world.committed().await;
    assert_eq!(after.0, before.0 + 1);
    assert_ne!(after.1, before.1);
    assert_ne!(after.2, before.2);
    assert_eq!(world.snapshot().await.lens_weights, real_weights());
}

/// A lens a weight map does not name weighs zero, so padding a set with zero
/// weights, or dropping its zero weights, spells the same set. Through the
/// public mailbox, in both directions: the world's set is sparse and a padded
/// twin is refused as `NoCanonicalChange`; the world's set is padded and a
/// sparse twin is refused the same way. Each refusal leaves revision, digests
/// and operator log unchanged and the stored spelling as it was. A set that
/// really moves a weight then commits, so the refusals are not an inert harness.
#[tokio::test]
async fn a_lens_set_spelled_with_or_without_its_zero_weights_commits_nothing() {
    let world = World::active().await;
    let set = |entries: &[(Lens, u32)]| LensWeights::new(entries.iter().copied().collect());
    let submit = |weights: LensWeights| {
        let world = &world;
        async move {
            let snapshot = world.snapshot().await;
            world
                .mailbox
                .submit_principal(
                    PrincipalCommandIntent {
                        id: CommandId::new(),
                        world_id: snapshot.world_id,
                        expected_revision: snapshot.revision,
                        body: CommandBody::SetLensWeights { weights },
                    },
                    &world.principal,
                )
                .await
        }
    };
    let refused_as_no_change = |weights: LensWeights, stored: LensWeights| {
        let world = &world;
        let submit = &submit;
        async move {
            let before = world.committed().await;
            let result = submit(weights.clone()).await;
            let Err(MailboxError::Kernel(KernelError::PatchRejected(mismatches))) = &result else {
                panic!("{weights:?} over {stored:?} was not refused as no change: {result:?}");
            };
            assert_eq!(mismatches, &vec![Mismatch::NoCanonicalChange]);
            assert_eq!(world.committed().await, before, "{weights:?} committed");
            assert_eq!(world.snapshot().await.lens_weights, stored);
        }
    };

    // Sparse on the world, padded in the command. The world was created with
    // `{patina: 1}`.
    let sparse = set(&[(Lens::Patina, 1)]);
    assert_eq!(world.snapshot().await.lens_weights, sparse);
    refused_as_no_change(set(&[(Lens::Patina, 1), (Lens::Charter, 0)]), sparse.clone()).await;
    let every = LensWeights::new(
        Lens::ALL
            .into_iter()
            .map(|lens| (lens, u32::from(lens == Lens::Patina)))
            .collect(),
    );
    assert_eq!(every.iter().count(), 8);
    refused_as_no_change(every, sparse.clone()).await;

    // A real change commits, and stores its own spelling: padded.
    let before = world.committed().await;
    let padded = set(&[(Lens::Patina, 0), (Lens::Charter, 3)]);
    let receipt = submit(padded.clone()).await.expect("moving weight from patina to charter commits");
    assert!(matches!(receipt, SubmitReceipt::Applied(_)));
    assert_eq!(world.committed().await.0, before.0 + 1);
    assert_eq!(world.snapshot().await.lens_weights, padded);

    // Padded on the world, sparse in the command.
    refused_as_no_change(set(&[(Lens::Charter, 3)]), padded.clone()).await;
    refused_as_no_change(
        set(&[(Lens::Charter, 3), (Lens::Veil, 0), (Lens::Numen, 0)]),
        padded.clone(),
    )
    .await;

    // A set that moves one weight by one is a change.
    let before = world.committed().await;
    let moved = set(&[(Lens::Charter, 3), (Lens::Numen, 1)]);
    submit(moved.clone()).await.expect("adding weight to numen commits");
    assert_eq!(world.committed().await.0, before.0 + 1);
    assert_eq!(world.snapshot().await.lens_weights, moved);
}

/// A lens name the library does not know cannot be decoded into a command
/// body, beside names it does. This is a serde limit, stated as one: the body
/// never exists, so no kernel admission is being proven here.
#[test]
fn set_lens_weights_with_an_unknown_lens_does_not_decode() {
    let refused = serde_json::from_value::<CommandBody>(json!({
        "type": "set_lens_weights",
        "weights": {"patina": 1, "veil ": 1},
    }));
    assert!(refused.is_err(), "an unknown lens decoded: {refused:?}");
    let accepted = serde_json::from_value::<CommandBody>(json!({
        "type": "set_lens_weights",
        "weights": {"patina": 1, "veil": 1},
    }))
    .expect("known lens names decode");
    assert!(matches!(accepted, CommandBody::SetLensWeights { .. }));
}
