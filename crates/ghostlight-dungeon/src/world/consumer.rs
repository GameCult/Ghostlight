//! The consumer ingress: the world's third patch author.
//!
//! This module owns exactly four things — decoding a document, bounding it,
//! authenticating the consumer that sent it, and projecting one receipt. It
//! owns no world truth, holds no opinion about a patch's content, and does not
//! pre-validate: a second reducer here could disagree with the kernel's, so
//! structure is decided once, by `resolve_patch`, behind the same
//! `require_patch_author`, `confine_to_ground`, and idempotency ledger every
//! other author passes through.

use super::mailbox::{ConsumerPort, MailboxError};
use super::patch::{self, PatchDecodeError};
use super::{
    CommandId, ConsumerId, KernelError, Mismatch, PatchAnswer, SubmitReceipt, WorldId, WorldPatch,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

/// The inbound document and the outbound receipt. Ghostlight-owned typed
/// records in canonical MessagePack, not CultNet control messages.
pub(crate) const CONSUMER_PATCH_SCHEMA: &str = "ghostlight.consumer_patch.v0";
pub(crate) const CONSUMER_RECEIPT_SCHEMA: &str = "ghostlight.consumer_receipt.v0";

/// The one namespace a consumer's command key is derived under.
const CONSUMER_COMMAND_NAMESPACE: &str = "ghostlight.consumer.command.v0";

/// The envelope's own fields, generously bounded. The patch bound is
/// `patch::MAX_PATCH_BYTES` and stays there; this is derived from it so the
/// transport guard cannot become a second opinion about how big a patch may be.
const CONSUMER_ENVELOPE_SLACK: usize = 8 * 1024;
pub(crate) const CONSUMER_BODY_LIMIT: usize = patch::MAX_PATCH_BYTES + CONSUMER_ENVELOPE_SLACK;

/// The environment variable naming the credentials file.
pub(crate) const CONSUMER_CREDENTIALS_ENVIRONMENT: &str = "GHOSTLIGHT_CONSUMER_CREDENTIALS";

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConsumerPatchDocument {
    /// Pinned to `CONSUMER_PATCH_SCHEMA`; any other value is refused before the
    /// body is read.
    pub(crate) schema: String,
    pub(crate) world_id: WorldId,
    /// The consumer's configured name. Lowered to a `ConsumerId` by the
    /// registry and never carried further; the kernel never sees it.
    pub(crate) consumer: String,
    /// Presented secret. Compared against the registry's stored SHA-256 and
    /// dropped. Never logged, never journaled, never echoed.
    pub(crate) secret: String,
    /// The consumer's own retry key. With the world and the consumer, this is
    /// the whole command-id derivation.
    pub(crate) idempotency_key: String,
    /// The Ghostlight revision this document was built against.
    pub(crate) expected_revision: u64,
    /// Absent for the ordinary component-only batch; present only when the
    /// document declares in Active.
    pub(crate) answers: Option<PatchAnswer>,
    /// Canonical-MessagePack `WorldPatch` frame, decoded by `decode_patch`.
    /// A nested frame rather than an inline value, so `MAX_PATCH_BYTES` is
    /// checked against the patch itself before any of its items deserializes.
    #[serde(with = "serde_bytes")]
    pub(crate) patch: Vec<u8>,
}

/// `derive(Debug)` would print `secret` in plaintext into any log, panic
/// message, or test failure that formats this document. `Debug` is for
/// diagnostics, not custody, so the field is redacted here rather than left to
/// callers to remember not to print it.
impl std::fmt::Debug for ConsumerPatchDocument {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ConsumerPatchDocument")
            .field("schema", &self.schema)
            .field("world_id", &self.world_id)
            .field("consumer", &self.consumer)
            .field("secret", &"<redacted>")
            .field("idempotency_key", &self.idempotency_key)
            .field("expected_revision", &self.expected_revision)
            .field("answers", &self.answers)
            .field("patch", &self.patch)
            .finish()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConsumerReceiptDocument {
    pub(crate) schema: String,
    pub(crate) world_id: Option<WorldId>,
    /// Absent when the document did not decode far enough to derive one. The
    /// derivation reads the world, the consumer, and the idempotency key, so a
    /// frame that fails before authentication has no command key to name.
    pub(crate) command_id: Option<CommandId>,
    /// The live revision whenever the kernel was reached, applied or not.
    pub(crate) revision: Option<u64>,
    pub(crate) outcome: ConsumerOutcome,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub(crate) enum ConsumerOutcome {
    Applied {
        state_digest: String,
        commit_digest: String,
    },
    AlreadyApplied {
        state_digest: String,
        commit_digest: String,
    },
    /// One verdict for one document. `mismatches` is the resolver's complete
    /// set when the refusal is structural, and empty otherwise. A consumer
    /// wanting per-item results splits documents.
    Refused {
        gate: ConsumerRefusal,
        mismatches: Vec<Mismatch>,
    },
    /// The owner task is restarting. The same bytes may be sent again.
    Unavailable,
    /// The reply channel dropped. Resubmit the same bytes: the derived command
    /// id and the ledger make the resubmission the probe.
    Unknown,
}

/// A wire projection of `KernelError` and `PatchDecodeError`. It decides
/// nothing, and its mapping is exhaustive so a new kernel error cannot silently
/// become `Internal`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ConsumerRefusal {
    Schema,
    TooLarge,
    /// The byte-decoded patch held more declarations than
    /// `patch::MAX_PATCH_DECLARATIONS`.
    TooManyDeclarations,
    /// The byte-decoded patch held more operations than
    /// `patch::MAX_PATCH_OPERATIONS`.
    TooManyOperations,
    /// The byte-decoded patch cited more evidence than
    /// `patch::MAX_PATCH_EVIDENCE`.
    TooManyEvidence,
    NotCanonical,
    Malformed,
    Unauthenticated,
    Unauthorized,
    WorldMismatch,
    StaleRevision,
    CommandIdConflict,
    Structural,
    AnswerRequired,
    AnswerNotDerived,
    AnswerNotSatisfied,
    Internal,
}

/// Configured consumers, keyed by name, valued by the SHA-256 of that
/// consumer's shared secret. The plaintext never enters the process image.
///
/// A missing file means no consumers and every document is `Unauthenticated`.
/// That is the fail-closed default, and it is why this door can exist before an
/// operator has configured anything.
#[derive(Clone, Debug, Default)]
pub(crate) struct ConsumerRegistry {
    entries: BTreeMap<String, [u8; 32]>,
}

impl ConsumerRegistry {
    pub(crate) fn empty() -> Self {
        Self::default()
    }

    /// One `name = <64 hex digits>` line per consumer, blank lines and `#`
    /// comments ignored. A file that does not exist yields no consumers; a file
    /// that exists and is malformed is an error, because a mistyped credential
    /// file must not read as an empty one.
    pub(crate) fn from_secret_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::empty());
        }
        let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
        let mut entries = BTreeMap::new();
        for (number, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((name, digest)) = line.split_once('=') else {
                return Err(format!(
                    "consumer credential line {} has no `=`",
                    number + 1
                ));
            };
            let name = name.trim();
            let digest = decode_digest(digest.trim()).ok_or_else(|| {
                format!(
                    "consumer credential line {} is not a sha256 hex digest",
                    number + 1
                )
            })?;
            if name.is_empty() || entries.insert(name.to_owned(), digest).is_some() {
                return Err(format!(
                    "consumer credential line {} names an empty or duplicate consumer",
                    number + 1
                ));
            }
        }
        Ok(Self { entries })
    }

    #[cfg(test)]
    pub(crate) fn with_secret(name: &str, secret: &str) -> Self {
        Self {
            entries: BTreeMap::from([(name.to_owned(), Sha256::digest(secret.as_bytes()).into())]),
        }
    }

    /// The presented secret is hashed and compared in constant time, then
    /// dropped. A name that is not configured takes the same comparison against
    /// a zero digest, so a miss and a wrong secret are one answer.
    fn authenticate(&self, name: &str, secret: &str) -> Option<ConsumerId> {
        let presented: [u8; 32] = Sha256::digest(secret.as_bytes()).into();
        let stored = self.entries.get(name).copied().unwrap_or([0u8; 32]);
        let mut difference = 0u8;
        for (left, right) in presented.iter().zip(stored.iter()) {
            difference |= left ^ right;
        }
        if difference == 0 && self.entries.contains_key(name) {
            Some(ConsumerId::of_name(name))
        } else {
            None
        }
    }
}

fn decode_digest(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 {
        return None;
    }
    let mut digest = [0u8; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(index * 2..index * 2 + 2)?, 16).ok()?;
    }
    Some(digest)
}

/// Decode, bound, authenticate, submit, project. One atomic verdict per
/// document: there is no partial acceptance, because there is one command.
pub(crate) async fn admit_document(
    port: &ConsumerPort,
    registry: &ConsumerRegistry,
    bytes: &[u8],
) -> ConsumerReceiptDocument {
    let document = match decode_document(bytes) {
        Ok(document) => document,
        Err(gate) => return refused(None, None, None, gate),
    };
    if document.schema != CONSUMER_PATCH_SCHEMA {
        return refused(None, None, None, ConsumerRefusal::Schema);
    }
    let world_id = Some(document.world_id);
    let Some(consumer) = registry.authenticate(&document.consumer, &document.secret) else {
        return refused(world_id, None, None, ConsumerRefusal::Unauthenticated);
    };
    let command_id = CommandId::derived(
        CONSUMER_COMMAND_NAMESPACE,
        &[
            &document.world_id.text(),
            &consumer.text(),
            &document.idempotency_key,
        ],
    );
    let patch = match patch::decode_patch(&document.patch) {
        Ok(patch) => patch,
        Err(error) => {
            return refused(world_id, Some(command_id), None, decode_refusal(error));
        }
    };
    let outcome = port
        .submit_consumer(
            document.world_id,
            document.expected_revision,
            command_id,
            consumer,
            document.answers.clone(),
            patch,
        )
        .await;
    project(world_id, command_id, outcome)
}

/// The outer frame's own decode: bounded, canonical, and tiny. It runs before
/// the nested patch frame is looked at, and the nested frame carries its own
/// bound.
fn decode_document(bytes: &[u8]) -> Result<ConsumerPatchDocument, ConsumerRefusal> {
    if bytes.len() > CONSUMER_BODY_LIMIT {
        return Err(ConsumerRefusal::TooLarge);
    }
    let document: ConsumerPatchDocument =
        rmp_serde::from_slice(bytes).map_err(|_| ConsumerRefusal::Malformed)?;
    if rmp_serde::to_vec_named(&document).map_err(|_| ConsumerRefusal::Malformed)? != bytes {
        return Err(ConsumerRefusal::NotCanonical);
    }
    Ok(document)
}

fn decode_refusal(error: PatchDecodeError) -> ConsumerRefusal {
    match error {
        PatchDecodeError::TooLarge { .. } => ConsumerRefusal::TooLarge,
        PatchDecodeError::TooManyDeclarations { .. } => ConsumerRefusal::TooManyDeclarations,
        PatchDecodeError::TooManyOperations { .. } => ConsumerRefusal::TooManyOperations,
        PatchDecodeError::TooManyEvidence { .. } => ConsumerRefusal::TooManyEvidence,
        PatchDecodeError::NotCanonical => ConsumerRefusal::NotCanonical,
        PatchDecodeError::Malformed => ConsumerRefusal::Malformed,
    }
}

fn project(
    world_id: Option<WorldId>,
    command_id: CommandId,
    outcome: Result<SubmitReceipt, MailboxError>,
) -> ConsumerReceiptDocument {
    let (revision, outcome) = match outcome {
        Ok(SubmitReceipt::Applied(receipt)) => (
            Some(receipt.resulting_revision),
            ConsumerOutcome::Applied {
                state_digest: receipt.resulting_state_digest,
                commit_digest: receipt.commit_digest,
            },
        ),
        Ok(SubmitReceipt::AlreadyApplied(receipt)) => (
            Some(receipt.resulting_revision),
            ConsumerOutcome::AlreadyApplied {
                state_digest: receipt.resulting_state_digest,
                commit_digest: receipt.commit_digest,
            },
        ),
        Err(MailboxError::Unavailable) => (None, ConsumerOutcome::Unavailable),
        Err(MailboxError::OutcomeUnknown { .. }) => (None, ConsumerOutcome::Unknown),
        Err(MailboxError::Kernel(error)) => {
            let revision = match &error {
                KernelError::RevisionMismatch { actual, .. } => Some(*actual),
                _ => None,
            };
            let mismatches = match &error {
                KernelError::PatchRejected(set) => set.clone(),
                _ => Vec::new(),
            };
            (
                revision,
                ConsumerOutcome::Refused {
                    gate: kernel_refusal(&error),
                    mismatches,
                },
            )
        }
    };
    ConsumerReceiptDocument {
        schema: CONSUMER_RECEIPT_SCHEMA.into(),
        world_id,
        command_id: Some(command_id),
        revision,
        outcome,
    }
}

/// Exhaustive by construction: a new `KernelError` variant fails to compile
/// here rather than becoming `Internal` by default.
fn kernel_refusal(error: &KernelError) -> ConsumerRefusal {
    match error {
        KernelError::PatchRejected(_) => ConsumerRefusal::Structural,
        KernelError::WorldMismatch | KernelError::OpenedWorldMismatch => {
            ConsumerRefusal::WorldMismatch
        }
        KernelError::Unauthorized
        | KernelError::AuthenticationMismatch
        | KernelError::ControllerMismatch
        | KernelError::NotDraftApprover => ConsumerRefusal::Unauthorized,
        KernelError::RevisionMismatch { .. } => ConsumerRefusal::StaleRevision,
        KernelError::CommandIdConflict | KernelError::CreationConflict => {
            ConsumerRefusal::CommandIdConflict
        }
        KernelError::AnswerRequired => ConsumerRefusal::AnswerRequired,
        KernelError::AnswerNotDerived => ConsumerRefusal::AnswerNotDerived,
        KernelError::AnswerNotSatisfied => ConsumerRefusal::AnswerNotSatisfied,
        KernelError::InvalidCommandId
        | KernelError::EmptyTitle
        | KernelError::EmptyPrincipal
        | KernelError::ActionRejected(_)
        | KernelError::WrongPhase { .. }
        | KernelError::DraftAlreadyApproved
        | KernelError::MissingApprovals(_)
        | KernelError::OpportunityMismatch
        | KernelError::ScopeChanged { .. }
        | KernelError::AffordanceDenied
        | KernelError::CreationTargetOccupied
        | KernelError::WorldNotCreated
        | KernelError::RecoveryRequired { .. }
        | KernelError::OwnershipLost
        | KernelError::Serialization(_)
        | KernelError::Store(_)
        | KernelError::CorruptJournal(_)
        | KernelError::Invariant(_) => ConsumerRefusal::Internal,
    }
}

fn refused(
    world_id: Option<WorldId>,
    command_id: Option<CommandId>,
    revision: Option<u64>,
    gate: ConsumerRefusal,
) -> ConsumerReceiptDocument {
    ConsumerReceiptDocument {
        schema: CONSUMER_RECEIPT_SCHEMA.into(),
        world_id,
        command_id,
        revision,
        outcome: ConsumerOutcome::Refused {
            gate,
            mismatches: Vec::new(),
        },
    }
}

/// The canonical encoding of one document, for the ingress's callers and for
/// the tests that build one.
#[cfg(test)]
pub(crate) fn encode_document(document: &ConsumerPatchDocument) -> Result<Vec<u8>, String> {
    rmp_serde::to_vec_named(document).map_err(|error| error.to_string())
}

pub(crate) fn encode_receipt(receipt: &ConsumerReceiptDocument) -> Result<Vec<u8>, String> {
    rmp_serde::to_vec_named(receipt).map_err(|error| error.to_string())
}

#[cfg(test)]
pub(crate) fn build_document(
    world_id: WorldId,
    consumer: &str,
    secret: &str,
    idempotency_key: &str,
    expected_revision: u64,
    answers: Option<PatchAnswer>,
    patch: &WorldPatch,
) -> Result<ConsumerPatchDocument, String> {
    Ok(ConsumerPatchDocument {
        schema: CONSUMER_PATCH_SCHEMA.into(),
        world_id,
        consumer: consumer.into(),
        secret: secret.into(),
        idempotency_key: idempotency_key.into(),
        expected_revision,
        answers,
        patch: rmp_serde::to_vec_named(patch).map_err(|error| error.to_string())?,
    })
}

#[cfg(test)]
mod tests {
    use super::super::DecisionInvocation;
    use super::super::tests::opportunity_for;
    use super::super::tests::{
        activate, auth_principal, command, creation, operations, owner, speak_entry, submit_owner,
    };
    use super::super::{
        AffordanceId, AuthenticatedCaller, AuthorityGrantRef, AuthorityKindName,
        AuthorityTargetRef, CallerId, CausalBoundary, CommandBody, CommandEnvelope, CommitmentKind,
        ComponentOp, CoverBudget, DecisionOpportunity, DecisionScope, Declaration, DraftHandle,
        EntityDeclaration, EntityId, EntityKind, EvidenceRef, FactDeclaration, FactStandingRef,
        FictionalMinutes, JurisdictionKey, NewController, PatchGround, Quantity, Ref,
        RouteDeclaration, Statement, SubjectDeclaration, SubjectId, SubjectKind, SystemCapability,
        WorldId, WorldKernel, WorldMailbox, WorldScaleIntentRef, agency_graph, derive_boundaries,
        derive_cover, derive_opportunities, derive_scale_deficit, ground_covers,
    };
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;

    const CONSUMER: &str = "mirror-consumer";
    /// The fixture generates its own credential. No real secret lives here, and
    /// the registry only ever holds its digest.
    const SECRET: &str = "fixture-consumer-secret-4f2a";
    const OTHER: &str = "other-consumer";
    const MIRROR_EVIDENCE: &str = "mirror:opening-count";
    const HOLD_KIND: &str = "hold";

    fn consumer() -> ConsumerId {
        ConsumerId::of_name(CONSUMER)
    }

    fn registry() -> ConsumerRegistry {
        ConsumerRegistry::with_secret(CONSUMER, SECRET)
    }

    /// Two mirrors bound to one consumer, one ordinary Ghostlight subject, one
    /// resource, and one commitment from the first mirror to the second — which
    /// is what derives a `MissingStructure` boundary the consumer may answer.
    struct Mirror {
        commons: EntityId,
        grain: EntityId,
        first: SubjectId,
        second: SubjectId,
        local: SubjectId,
    }

    fn mirror_patch(commons: EntityId, speak: Ref<AffordanceId>) -> WorldPatch {
        let subject = |handle: &str, label: &str, controller: NewController| {
            Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new(handle),
                label: label.into(),
                kind: SubjectKind::Institution,
                affordances: match &controller {
                    NewController::External { .. } => BTreeSet::new(),
                    _ => BTreeSet::from([speak.clone()]),
                },
                controller,
                position: Some(Ref::Existing(commons)),
            })
        };
        WorldPatch {
            declarations: vec![
                Declaration::Entity(EntityDeclaration {
                    handle: DraftHandle::new("grain"),
                    label: "Winter Grain".into(),
                    kind: EntityKind::Resource,
                    container: None,
                }),
                subject(
                    "first",
                    "The Sunk Hold",
                    NewController::External {
                        consumer: consumer(),
                    },
                ),
                subject(
                    "second",
                    "The Deeper Hold",
                    NewController::External {
                        consumer: consumer(),
                    },
                ),
                subject("local", "The Rhythm Hall", NewController::OperationalAgent),
            ],
            operations: vec![
                ComponentOp::Admit {
                    holder: Ref::Draft(DraftHandle::new("first")),
                    resource: Ref::Draft(DraftHandle::new("grain")),
                    qty: Quantity(9),
                    evidence: EvidenceRef::new(MIRROR_EVIDENCE),
                },
                ComponentOp::CreateCommitment {
                    subject: Ref::Draft(DraftHandle::new("first")),
                    counterparty: Some(Ref::Draft(DraftHandle::new("second"))),
                    kind: CommitmentKind::Obligation,
                    due: FictionalMinutes(600),
                    period: None,
                    checks: Vec::new(),
                },
                // A second unlitigable promise, this one held by an ordinary
                // Ghostlight subject: the foreign boundary a consumer may not
                // answer.
                ComponentOp::CreateCommitment {
                    subject: Ref::Draft(DraftHandle::new("local")),
                    counterparty: Some(Ref::Draft(DraftHandle::new("second"))),
                    kind: CommitmentKind::Obligation,
                    due: FictionalMinutes(600),
                    period: None,
                    checks: Vec::new(),
                },
            ],
            evidence: vec![EvidenceRef::new(MIRROR_EVIDENCE)],
        }
    }

    fn subject_named(kernel: &WorldKernel, label: &str) -> SubjectId {
        *kernel
            .state
            .subjects
            .iter()
            .find(|(_, subject)| subject.label == label)
            .expect("a declared subject")
            .0
    }

    fn entity_named(kernel: &WorldKernel, label: &str) -> EntityId {
        *kernel
            .state
            .entities
            .iter()
            .find(|(_, record)| record.label == label)
            .expect("a declared entity")
            .0
    }

    fn draft_kernel(path: &Path, title: &str) -> (WorldKernel, Mirror) {
        let mut kernel = WorldKernel::create(
            path.join("world.cc"),
            creation(CommandId::new(), title),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let before = kernel.snapshot().unwrap();
        let commons = entity_named(&kernel, "The Commons");
        let speak = speak_entry(&kernel);
        submit_owner(
            &mut kernel,
            &before,
            CommandBody::AdmitPatch {
                answers: None,
                patch: mirror_patch(commons, speak),
            },
        );
        let mirror = Mirror {
            commons,
            grain: entity_named(&kernel, "Winter Grain"),
            first: subject_named(&kernel, "The Sunk Hold"),
            second: subject_named(&kernel, "The Deeper Hold"),
            local: subject_named(&kernel, "The Rhythm Hall"),
        };
        (kernel, mirror)
    }

    fn mirror_kernel(path: &Path, title: &str) -> (WorldKernel, Mirror) {
        let (mut kernel, mirror) = draft_kernel(path, title);
        activate(&mut kernel);
        (kernel, mirror)
    }

    fn consumer_caller(id: ConsumerId) -> CallerId {
        CallerId::System(SystemCapability::Consumer { consumer: id })
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

    fn as_consumer(
        kernel: &mut WorldKernel,
        body: CommandBody,
    ) -> Result<SubmitReceipt, KernelError> {
        submit_as(kernel, consumer_caller(consumer()), body)
    }

    fn declaring(declarations: Vec<Declaration>) -> CommandBody {
        CommandBody::AdmitPatch {
            answers: None,
            patch: WorldPatch {
                declarations,
                operations: Vec::new(),
                evidence: Vec::new(),
            },
        }
    }

    fn outside(result: &Result<SubmitReceipt, KernelError>) -> bool {
        matches!(
            result,
            Err(KernelError::PatchRejected(set))
                if !set.is_empty()
                    && set
                        .iter()
                        .all(|mismatch| matches!(mismatch, Mismatch::OutsideJurisdiction { .. }))
        )
    }

    fn missing_structure(kernel: &WorldKernel, subject: SubjectId) -> CausalBoundary {
        derive_boundaries(&kernel.state)
            .unwrap()
            .into_iter()
            .find(|boundary| {
                matches!(
                    boundary,
                    CausalBoundary::MissingStructure { subject: named, .. } if *named == subject
                )
            })
            .expect("the fixture derives this boundary")
    }

    /// Granting the counterparty command over the promisor clears a
    /// `MissingStructure` boundary. Both subjects are the consumer's mirrors, so
    /// the whole answer sits inside its own ground.
    fn clearing_grant(mirror: &Mirror) -> Vec<ComponentOp> {
        vec![ComponentOp::GrantAuthority {
            holder: Ref::Existing(mirror.second),
            grant: AuthorityGrantRef {
                kind: AuthorityKindName(HOLD_KIND.into()),
                over: AuthorityTargetRef::Subject(Ref::Existing(mirror.first)),
            },
        }]
    }

    fn consumer_ops(mirror: &Mirror, qty: u64) -> WorldPatch {
        WorldPatch {
            declarations: Vec::new(),
            operations: vec![ComponentOp::Consume {
                holder: Ref::Existing(mirror.first),
                resource: Ref::Existing(mirror.grain),
                qty: Quantity(qty),
            }],
            evidence: Vec::new(),
        }
    }

    // ---- the three negative proofs the consumer profile demands ---------

    #[test]
    fn ordinary_strategic_waves_cannot_select_an_external_mirror() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, mirror) = mirror_kernel(directory.path(), "Waves");
        let snapshot = kernel.snapshot().unwrap();

        let opportunities = derive_opportunities(&kernel.state).unwrap();
        let scopes: BTreeSet<SubjectId> = opportunities
            .iter()
            .map(|opportunity| opportunity.scope.subject_id)
            .collect();
        assert!(!scopes.contains(&mirror.first) && !scopes.contains(&mirror.second));
        assert!(scopes.contains(&mirror.local));

        let graph = agency_graph(&kernel.state);
        assert!(!graph.subjects.contains(&mirror.first));
        let cover = derive_cover(
            snapshot.world_id,
            snapshot.now,
            30,
            &opportunities,
            &graph,
            CoverBudget {
                cells: 8,
                constituent_cap: 8,
                urgency_slots: 8,
            },
        );
        assert!(
            !cover
                .cells
                .iter()
                .flat_map(|cell| cell.members().iter())
                .any(|constituent| constituent.subject == mirror.first)
        );

        // The mirror is an ordinary subject in the snapshot throughout, with no
        // controller and no turn.
        let seen = snapshot
            .subjects
            .iter()
            .find(|subject| subject.id == mirror.first)
            .expect("the mirror is visible");
        assert_eq!(seen.controller_mode, None);
        assert_eq!(seen.controller_id, None);
        assert!(seen.affordances.is_empty());

        // A hand-built opportunity naming the mirror's scope reaches the
        // controller lane and is refused there.
        let borrowed = opportunities.first().expect("an ordinary opportunity");
        let forged = DecisionOpportunity {
            scope: DecisionScope {
                subject_id: mirror.first,
            },
            ..borrowed.clone()
        };
        let error = submit_as(
            &mut kernel,
            CallerId::Controller(borrowed.controller_id),
            CommandBody::DeclineDecision {
                opportunity: forged,
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, KernelError::OpportunityMismatch),
            "{error:?}"
        );
    }

    #[test]
    fn a_malformed_or_stale_batch_commits_nothing() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, mirror) = mirror_kernel(directory.path(), "Stale");
        let before = kernel.snapshot().unwrap();
        let patch = consumer_ops(&mirror, 1);

        // (a) bytes that are not canonical MessagePack, and bytes that are not
        // MessagePack at all.
        let noncanonical = rmp_serde::to_vec(&patch).unwrap();
        assert!(
            matches!(
                patch::decode_patch(&noncanonical),
                Err(PatchDecodeError::NotCanonical | PatchDecodeError::Malformed)
            ),
            "a non-canonical frame decoded"
        );
        assert_eq!(
            patch::decode_patch(&noncanonical[..3]),
            Err(PatchDecodeError::Malformed)
        );

        // (b) a frame one byte over the cap.
        let oversize = vec![0u8; patch::MAX_PATCH_BYTES + 1];
        assert_eq!(
            patch::decode_patch(&oversize),
            Err(PatchDecodeError::TooLarge {
                bytes: patch::MAX_PATCH_BYTES + 1
            })
        );

        // (c) a revision one behind, after an intervening commit.
        let intervening = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &intervening,
            operations(consumer_ops(&mirror, 1).operations),
        );
        let after_intervening = kernel.snapshot().unwrap();
        let stale = kernel
            .submit(
                CommandEnvelope {
                    id: CommandId::new(),
                    world_id: after_intervening.world_id,
                    expected_revision: after_intervening.revision - 1,
                    caller: consumer_caller(consumer()),
                    body: CommandBody::AdmitPatch {
                        answers: None,
                        patch: patch.clone(),
                    },
                },
                &AuthenticatedCaller::fixture(consumer_caller(consumer())),
            )
            .unwrap_err();
        let KernelError::RevisionMismatch { actual, .. } = stale else {
            panic!("{stale:?}")
        };
        assert_eq!(actual, after_intervening.revision);

        // (d) a structurally invalid patch: the complete mismatch set, nothing
        // committed.
        let guarded = kernel.snapshot().unwrap();
        let broken = as_consumer(
            &mut kernel,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: Vec::new(),
                    operations: vec![ComponentOp::Consume {
                        holder: Ref::Draft(DraftHandle::new("nowhere")),
                        resource: Ref::Existing(mirror.grain),
                        qty: Quantity(1),
                    }],
                    evidence: Vec::new(),
                },
            },
        )
        .unwrap_err();
        assert!(
            matches!(broken, KernelError::PatchRejected(_)),
            "{broken:?}"
        );
        let after = kernel.snapshot().unwrap();
        assert_eq!(after.revision, guarded.revision);
        assert_eq!(after.state_digest, guarded.state_digest);
        assert_ne!(after.state_digest, before.state_digest);
    }

    #[test]
    fn foreign_effects_become_local_consequences_only_after_the_receipt() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, mirror) = mirror_kernel(directory.path(), "Custody");
        let transfer = vec![ComponentOp::Transfer {
            from: Ref::Existing(mirror.first),
            to: Ref::Existing(mirror.local),
            resource: Ref::Existing(mirror.grain),
            qty: Quantity(2),
        }];
        let before = kernel.state.holdings.clone();
        let refused = as_consumer(&mut kernel, operations(transfer.clone()));
        assert!(outside(&refused), "{refused:?}");
        assert_eq!(kernel.state.holdings, before);

        // The same transfer through the world owner commits, so the refusal is
        // the consumer's confinement rather than a broken operation.
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(&mut kernel, &snapshot, operations(transfer));
        assert_ne!(kernel.state.holdings, before);
    }

    // ---- end to end ------------------------------------------------------

    async fn submit_through(
        mailbox: &WorldMailbox,
        principal: crate::world::PrincipalId,
        body: CommandBody,
    ) -> SubmitReceipt {
        let snapshot = mailbox.snapshot().await.unwrap();
        mailbox
            .submit_fixture(
                CommandEnvelope {
                    id: CommandId::new(),
                    world_id: snapshot.world_id,
                    expected_revision: snapshot.revision,
                    caller: CallerId::Principal(principal.clone()),
                    body,
                },
                &auth_principal(principal),
            )
            .await
            .expect("the owner lane commits")
    }

    /// The same fixture world, authored through the mailbox rather than the
    /// kernel, because the ingress talks to the owner task and nothing else.
    async fn mirror_mailbox(
        path: &Path,
    ) -> (
        WorldMailbox,
        tokio::task::JoinHandle<()>,
        ConsumerPort,
        Mirror,
        WorldId,
    ) {
        let (mailbox, task) = WorldMailbox::open(path.join("world.cc")).expect("an empty world");
        mailbox
            .create_fixture(
                creation(CommandId::new(), "Ingress"),
                &auth_principal(owner()),
            )
            .await
            .expect("a created world");
        let genesis = mailbox.snapshot().await.unwrap();
        let commons = genesis
            .places
            .iter()
            .find(|place| place.label == "The Commons")
            .expect("the genesis place")
            .id;
        let speak = Ref::Existing(
            genesis
                .affordances
                .iter()
                .find(|entry| entry.entry.kind.0 == "speak")
                .expect("the kernel Speak entry")
                .id,
        );
        submit_through(
            &mailbox,
            owner(),
            CommandBody::AdmitPatch {
                answers: None,
                patch: mirror_patch(commons, speak),
            },
        )
        .await;
        submit_through(&mailbox, owner(), CommandBody::ApproveDraft).await;
        submit_through(
            &mailbox,
            crate::world::tests::player(),
            CommandBody::ApproveDraft,
        )
        .await;
        submit_through(&mailbox, owner(), CommandBody::ActivateWorld).await;

        let active = mailbox.snapshot().await.unwrap();
        let subject = |label: &str| {
            active
                .subjects
                .iter()
                .find(|subject| subject.label == label)
                .expect("a declared subject")
                .id
        };
        let mirror = Mirror {
            commons,
            grain: active
                .resources
                .iter()
                .find(|resource| resource.label == "Winter Grain")
                .expect("the declared resource")
                .id,
            first: subject("The Sunk Hold"),
            second: subject("The Deeper Hold"),
            local: subject("The Rhythm Hall"),
        };
        let port = ConsumerPort::new(mailbox.clone());
        let world_id = active.world_id;
        (mailbox, task, port, mirror, world_id)
    }

    #[tokio::test]
    async fn inbound_bytes_reach_a_committed_effect_and_a_receipt() {
        let directory = tempfile::tempdir().unwrap();
        let (mailbox, task, port, mirror, world_id) = mirror_mailbox(directory.path()).await;
        let before = mailbox.snapshot().await.unwrap();
        let bytes = encode_document(
            &build_document(
                world_id,
                CONSUMER,
                SECRET,
                "batch-1",
                before.revision,
                None,
                &consumer_ops(&mirror, 3),
            )
            .unwrap(),
        )
        .unwrap();

        let receipt = admit_document(&port, &registry(), &bytes).await;
        let decoded: ConsumerReceiptDocument =
            rmp_serde::from_slice(&encode_receipt(&receipt).unwrap()).unwrap();
        assert_eq!(decoded.schema, CONSUMER_RECEIPT_SCHEMA);
        assert!(
            matches!(decoded.outcome, ConsumerOutcome::Applied { .. }),
            "{:?}",
            decoded.outcome
        );
        let after = mailbox.snapshot().await.unwrap();
        assert_eq!(after.revision, before.revision + 1);
        assert_eq!(decoded.revision, Some(after.revision));

        // The port holds its own clone of the mailbox: the owner task ends
        // when the last sender does.
        drop(port);
        drop(mailbox);
        task.await.unwrap();
        // Replaying the journal reproduces the same state digest, and the
        // committed effect moved the mirror's components.
        let replayed = WorldKernel::open(directory.path().join("world.cc"), world_id).unwrap();
        let replayed_snapshot = replayed.snapshot().unwrap();
        assert_eq!(replayed_snapshot.state_digest, after.state_digest);
        assert_eq!(
            replayed_snapshot
                .subjects
                .iter()
                .find(|subject| subject.id == mirror.first)
                .and_then(|subject| subject.components.holdings.get(&mirror.grain).copied()),
            Some(Quantity(6))
        );
    }

    #[tokio::test]
    async fn an_identical_resubmission_returns_the_original_receipt_and_a_new_body_conflicts() {
        let directory = tempfile::tempdir().unwrap();
        let (mailbox, task, port, mirror, world_id) = mirror_mailbox(directory.path()).await;
        let revision = mailbox.snapshot().await.unwrap().revision;
        let bytes = encode_document(
            &build_document(
                world_id,
                CONSUMER,
                SECRET,
                "batch-1",
                revision,
                None,
                &consumer_ops(&mirror, 3),
            )
            .unwrap(),
        )
        .unwrap();
        let first = admit_document(&port, &registry(), &bytes).await;
        let ConsumerOutcome::Applied {
            commit_digest,
            state_digest,
        } = first.outcome.clone()
        else {
            panic!("{:?}", first.outcome)
        };

        // The same bytes at a moved revision: the ledger answers before the
        // revision check, so resubmission is the probe.
        let moved = mailbox.snapshot().await.unwrap().revision;
        assert_ne!(moved, revision);
        let again = admit_document(&port, &registry(), &bytes).await;
        assert_eq!(
            again.outcome,
            ConsumerOutcome::AlreadyApplied {
                commit_digest,
                state_digest,
            }
        );
        assert_eq!(mailbox.snapshot().await.unwrap().revision, moved);

        // A different body under the same key conflicts and commits nothing.
        let conflicting = encode_document(
            &build_document(
                world_id,
                CONSUMER,
                SECRET,
                "batch-1",
                moved,
                None,
                &consumer_ops(&mirror, 1),
            )
            .unwrap(),
        )
        .unwrap();
        let refused = admit_document(&port, &registry(), &conflicting).await;
        assert_eq!(
            refused.outcome,
            ConsumerOutcome::Refused {
                gate: ConsumerRefusal::CommandIdConflict,
                mismatches: Vec::new(),
            }
        );
        assert_eq!(mailbox.snapshot().await.unwrap().revision, moved);

        // The port holds its own clone of the mailbox: the owner task ends
        // when the last sender does.
        drop(port);
        drop(mailbox);
        task.await.unwrap();
    }

    // ---- caller and capability -------------------------------------------

    #[test]
    fn consumer_cannot_approve_activate_exercise_decline_or_advance_time() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, mirror) = mirror_kernel(directory.path(), "Capability");
        let opportunity = derive_opportunities(&kernel.state)
            .unwrap()
            .into_iter()
            .find(|value| value.scope.subject_id == mirror.local)
            .expect("the local subject has an opportunity");
        for body in [
            CommandBody::ApproveDraft,
            CommandBody::ActivateWorld,
            CommandBody::DeclineDecision {
                opportunity: opportunity.clone(),
            },
            CommandBody::AdvanceTime {
                minutes: super::super::TickMinutes::new(30).unwrap(),
            },
        ] {
            let error = as_consumer(&mut kernel, body).unwrap_err();
            assert!(matches!(error, KernelError::Unauthorized), "{error:?}");
        }
    }

    #[test]
    fn elaborator_and_clock_cannot_write_a_consumer_ground() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, mirror) = mirror_kernel(directory.path(), "Lanes");
        let body = operations(consumer_ops(&mirror, 1).operations);
        for caller in [
            CallerId::System(SystemCapability::Clock),
            CallerId::System(SystemCapability::Elaborator {
                jurisdiction: JurisdictionKey::PlaceSubtree(mirror.commons),
            }),
        ] {
            let error = submit_as(&mut kernel, caller, body.clone()).unwrap_err();
            assert!(matches!(error, KernelError::Unauthorized), "{error:?}");
        }
    }

    #[test]
    fn an_unbound_consumer_can_write_nothing_but_its_own_binding() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, mirror) = mirror_kernel(directory.path(), "Unbound");
        let stranger = ConsumerId::of_name(OTHER);
        let refused = submit_as(
            &mut kernel,
            consumer_caller(stranger),
            operations(consumer_ops(&mirror, 1).operations),
        );
        assert!(outside(&refused), "{refused:?}");

        // In Draft, the same unbound consumer may declare its own mirror and
        // nothing else. That is the whole bootstrap.
        let draft = tempfile::tempdir().unwrap();
        let mut fresh = WorldKernel::create(
            draft.path().join("world.cc"),
            creation(CommandId::new(), "Bootstrap"),
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let commons = entity_named(&fresh, "The Commons");
        let own = |consumer| {
            declaring(vec![Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new("mine"),
                label: "The Bound Hold".into(),
                kind: SubjectKind::Institution,
                controller: NewController::External { consumer },
                affordances: BTreeSet::new(),
                position: Some(Ref::Existing(commons)),
            })])
        };
        // A binding to another consumer is outside its ground.
        let foreign = submit_as(&mut fresh, consumer_caller(stranger), own(consumer()));
        assert!(outside(&foreign), "{foreign:?}");
        // Its own binding commits.
        submit_as(&mut fresh, consumer_caller(stranger), own(stranger))
            .expect("a consumer may declare its own mirror");
    }

    #[test]
    fn apply_effect_re_decides_consumer_authority() {
        let directory = tempfile::tempdir().unwrap();
        let (kernel, mirror) = draft_kernel(directory.path(), "Redecide");
        let command_id = CommandId::new();
        let foreign = WorldPatch {
            declarations: Vec::new(),
            operations: vec![ComponentOp::Admit {
                holder: Ref::Existing(mirror.local),
                resource: Ref::Existing(mirror.grain),
                qty: Quantity(1),
                evidence: EvidenceRef::new(MIRROR_EVIDENCE),
            }],
            evidence: vec![EvidenceRef::new(MIRROR_EVIDENCE)],
        };
        let resolved = patch::resolve_patch(&kernel.state, command_id, &foreign, None)
            .expect("the patch resolves");
        let effect = super::super::WorldEffect::PatchAdmitted {
            answers: None,
            resolved,
        };
        let mut candidate = kernel.state.clone();
        let error = super::super::apply_effect(
            &mut candidate,
            command_id,
            &consumer_caller(consumer()),
            &effect,
        )
        .unwrap_err();
        assert!(matches!(error, KernelError::Invariant(_)), "{error:?}");
        assert_eq!(candidate, kernel.state);
    }

    // ---- confinement, per row --------------------------------------------

    #[test]
    fn a_consumer_cannot_declare_a_place_a_route_or_a_canonical_fact() {
        let directory = tempfile::tempdir().unwrap();
        // Draft: the phase where declaring answers nothing, so what refuses
        // these three is the consumer's confinement and not the answer rule.
        let (mut kernel, mirror) = draft_kernel(directory.path(), "Declarations");
        let place = Declaration::Entity(EntityDeclaration {
            handle: DraftHandle::new("vault"),
            label: "The Deep Vault".into(),
            kind: EntityKind::Place,
            container: Some(Ref::Existing(mirror.commons)),
        });
        let route = Declaration::Route(RouteDeclaration {
            handle: DraftHandle::new("shaft"),
            label: "The Long Shaft".into(),
            from: Ref::Existing(mirror.commons),
            to: Ref::Draft(DraftHandle::new("vault")),
            access: super::super::AccessKind::Public,
            cost: super::super::Cost(3),
        });
        let canonical = Declaration::Fact(FactDeclaration {
            handle: DraftHandle::new("count"),
            label: "The Count".into(),
            statement: Statement::new("The hold is counted.").unwrap(),
            standing: FactStandingRef::Canonical {
                evidence: EvidenceRef::new(MIRROR_EVIDENCE),
            },
        });
        let cited = |declarations: Vec<Declaration>| CommandBody::AdmitPatch {
            answers: None,
            patch: WorldPatch {
                declarations,
                operations: Vec::new(),
                evidence: vec![EvidenceRef::new(MIRROR_EVIDENCE)],
            },
        };
        for declarations in [
            vec![place.clone()],
            vec![place.clone(), route.clone()],
            vec![canonical.clone()],
        ] {
            let refused = as_consumer(&mut kernel, cited(declarations));
            assert!(outside(&refused), "{refused:?}");
        }

        // The owner's identical patch commits, so the refusal is confinement
        // and not a broken declaration.
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: vec![place, route, canonical],
                    operations: Vec::new(),
                    evidence: vec![EvidenceRef::new(MIRROR_EVIDENCE)],
                },
            },
        );
    }

    #[test]
    fn a_consumer_cannot_relocate_open_close_or_alter_a_route() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, mirror) = draft_kernel(directory.path(), "Routes");
        // One route the owner declares, so every route operation has a live
        // referent to name.
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: vec![
                        Declaration::Entity(EntityDeclaration {
                            handle: DraftHandle::new("vault"),
                            label: "The Deep Vault".into(),
                            kind: EntityKind::Place,
                            container: None,
                        }),
                        Declaration::Route(RouteDeclaration {
                            handle: DraftHandle::new("shaft"),
                            label: "The Long Shaft".into(),
                            from: Ref::Existing(mirror.commons),
                            to: Ref::Draft(DraftHandle::new("vault")),
                            access: super::super::AccessKind::Public,
                            cost: super::super::Cost(3),
                        }),
                        Declaration::Route(RouteDeclaration {
                            handle: DraftHandle::new("gallery"),
                            label: "The Shut Gallery".into(),
                            from: Ref::Draft(DraftHandle::new("vault")),
                            to: Ref::Existing(mirror.commons),
                            access: super::super::AccessKind::Public,
                            cost: super::super::Cost(3),
                        }),
                    ],
                    // Declared shut, so `OpenRoute` names a change and
                    // `CloseRoute` on the shaft names one too.
                    operations: vec![ComponentOp::CloseRoute {
                        route: Ref::Draft(DraftHandle::new("gallery")),
                    }],
                    evidence: Vec::new(),
                },
            },
        );
        let edge = |label: &str| {
            *kernel
                .state
                .edges
                .iter()
                .find(|(_, record)| record.label() == label)
                .expect("the declared route")
                .0
        };
        let shaft = edge("The Long Shaft");
        let gallery = edge("The Shut Gallery");
        for operation in [
            ComponentOp::Relocate {
                subject: Ref::Existing(mirror.first),
                via: Ref::Existing(shaft),
            },
            ComponentOp::OpenRoute {
                route: Ref::Existing(gallery),
            },
            ComponentOp::CloseRoute {
                route: Ref::Existing(shaft),
            },
            ComponentOp::AlterCost {
                route: Ref::Existing(shaft),
                cost: super::super::Cost(9),
            },
        ] {
            let refused = as_consumer(&mut kernel, operations(vec![operation]));
            assert!(outside(&refused), "{refused:?}");
        }
    }

    #[test]
    fn a_consumer_admits_and_consumes_inside_its_own_custody() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, mirror) = draft_kernel(directory.path(), "Custody Positive");
        let receipt = as_consumer(
            &mut kernel,
            CommandBody::AdmitPatch {
                answers: None,
                patch: WorldPatch {
                    declarations: Vec::new(),
                    operations: vec![
                        ComponentOp::Admit {
                            holder: Ref::Existing(mirror.first),
                            resource: Ref::Existing(mirror.grain),
                            qty: Quantity(4),
                            evidence: EvidenceRef::new("mirror:second-count"),
                        },
                        ComponentOp::Consume {
                            holder: Ref::Existing(mirror.first),
                            resource: Ref::Existing(mirror.grain),
                            qty: Quantity(2),
                        },
                    ],
                    evidence: vec![EvidenceRef::new("mirror:second-count")],
                },
            },
        )
        .expect("a consumer writes inside its own custody");
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
        assert_eq!(
            kernel
                .state
                .holdings
                .get(&mirror.first)
                .and_then(|held| held.get(&mirror.grain))
                .copied(),
            Some(Quantity(11))
        );
    }

    // ---- separation and shape --------------------------------------------

    #[test]
    fn mismatch_never_appears_in_a_world_commit_but_does_appear_in_a_receipt() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, mirror) = mirror_kernel(directory.path(), "Separation");
        as_consumer(&mut kernel, operations(consumer_ops(&mirror, 1).operations))
            .expect("the consumer batch commits");
        let bytes = rmp_serde::to_vec_named(&kernel.state).expect("state encodes");
        let text = String::from_utf8_lossy(&bytes);
        for tag in [
            "outside_jurisdiction",
            "controller_grant_mismatch",
            "mismatch",
        ] {
            assert!(!text.contains(tag), "{tag} reached world state");
        }

        // The refusal receipt does carry them.
        let receipt = ConsumerReceiptDocument {
            schema: CONSUMER_RECEIPT_SCHEMA.into(),
            world_id: None,
            command_id: None,
            revision: None,
            outcome: ConsumerOutcome::Refused {
                gate: ConsumerRefusal::Structural,
                mismatches: vec![Mismatch::OutsideJurisdiction {
                    site: patch::Site::Operation(0),
                }],
            },
        };
        let encoded = encode_receipt(&receipt).unwrap();
        assert!(String::from_utf8_lossy(&encoded).contains("outside_jurisdiction"));
    }

    #[tokio::test]
    async fn no_consumer_secret_reaches_the_journal_the_receipt_or_a_log() {
        let directory = tempfile::tempdir().unwrap();
        let (mailbox, task, port, mirror, world_id) = mirror_mailbox(directory.path()).await;
        let revision = mailbox.snapshot().await.unwrap().revision;
        let bytes = encode_document(
            &build_document(
                world_id,
                CONSUMER,
                SECRET,
                "secret-batch",
                revision,
                None,
                &consumer_ops(&mirror, 2),
            )
            .unwrap(),
        )
        .unwrap();
        let receipt = admit_document(&port, &registry(), &bytes).await;
        assert!(!String::from_utf8_lossy(&encode_receipt(&receipt).unwrap()).contains(SECRET));
        assert!(!format!("{receipt:?}").contains(SECRET));
        // The port holds its own clone of the mailbox: the owner task ends
        // when the last sender does.
        drop(port);
        drop(mailbox);
        task.await.unwrap();
        let journal = std::fs::read(directory.path().join("world.cc")).unwrap();
        assert!(!String::from_utf8_lossy(&journal).contains(SECRET));
    }

    #[test]
    fn the_external_mirror_has_no_controller_and_no_affordance() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, mirror) = draft_kernel(directory.path(), "Pairing");
        let speak = speak_entry(&kernel);
        let declare = |controller: NewController, affordances: BTreeSet<Ref<AffordanceId>>| {
            declaring(vec![Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new("candidate"),
                label: "The Candidate".into(),
                kind: SubjectKind::Institution,
                controller,
                affordances,
                position: Some(Ref::Existing(mirror.commons)),
            })])
        };
        let pairing = |result: Result<SubmitReceipt, KernelError>| {
            matches!(
                result,
                Err(KernelError::PatchRejected(set))
                    if set
                        .iter()
                        .any(|mismatch| matches!(
                            mismatch,
                            Mismatch::ControllerGrantMismatch { .. }
                        ))
            )
        };
        let granted_mirror = submit_owner_result(
            &mut kernel,
            declare(
                NewController::External {
                    consumer: consumer(),
                },
                BTreeSet::from([speak.clone()]),
            ),
        );
        assert!(pairing(granted_mirror));
        let ungranted_ordinary = submit_owner_result(
            &mut kernel,
            declare(NewController::OperationalAgent, BTreeSet::new()),
        );
        assert!(pairing(ungranted_ordinary));
    }

    fn submit_owner_result(
        kernel: &mut WorldKernel,
        body: CommandBody,
    ) -> Result<SubmitReceipt, KernelError> {
        submit_as(kernel, CallerId::Principal(owner()), body)
    }

    // ---- bounds, one owner ------------------------------------------------

    #[test]
    fn the_patch_caps_have_one_owner() {
        let over_cap = WorldPatch {
            declarations: (0..=patch::MAX_PATCH_DECLARATIONS)
                .map(|index| {
                    Declaration::Entity(EntityDeclaration {
                        handle: DraftHandle::new(&format!("shed-{index}")),
                        label: format!("Shed {index}"),
                        kind: EntityKind::Place,
                        container: None,
                    })
                })
                .collect(),
            operations: Vec::new(),
            evidence: Vec::new(),
        };
        assert_eq!(
            patch::check_patch_caps(&over_cap),
            Err(PatchDecodeError::TooManyDeclarations {
                count: patch::MAX_PATCH_DECLARATIONS + 1
            })
        );
        let bytes = rmp_serde::to_vec_named(&over_cap).unwrap();
        assert_eq!(
            patch::decode_patch(&bytes),
            Err(PatchDecodeError::TooManyDeclarations {
                count: patch::MAX_PATCH_DECLARATIONS + 1
            })
        );
    }

    // ---- transport --------------------------------------------------------

    #[tokio::test]
    async fn a_document_is_refused_at_its_own_gate_before_the_kernel_is_reached() {
        let directory = tempfile::tempdir().unwrap();
        let (mailbox, task, port, mirror, world_id) = mirror_mailbox(directory.path()).await;
        let before = mailbox.snapshot().await.unwrap();
        let good = build_document(
            world_id,
            CONSUMER,
            SECRET,
            "gate",
            before.revision,
            None,
            &consumer_ops(&mirror, 1),
        )
        .unwrap();

        let gate_of = |receipt: &ConsumerReceiptDocument| match &receipt.outcome {
            ConsumerOutcome::Refused { gate, .. } => *gate,
            other => panic!("{other:?}"),
        };

        // An unknown schema string.
        let mut wrong_schema = good.clone();
        wrong_schema.schema = "ghostlight.consumer_patch.v99".into();
        let receipt =
            admit_document(&port, &registry(), &encode_document(&wrong_schema).unwrap()).await;
        assert_eq!(gate_of(&receipt), ConsumerRefusal::Schema);
        assert_eq!(receipt.command_id, None);

        // An unregistered consumer, a wrong secret, and a missing registry.
        let mut unregistered = good.clone();
        unregistered.consumer = OTHER.into();
        let mut wrong_secret = good.clone();
        wrong_secret.secret = "not-the-secret".into();
        for (document, registry) in [
            (unregistered, registry()),
            (wrong_secret, registry()),
            (good.clone(), ConsumerRegistry::empty()),
        ] {
            let receipt =
                admit_document(&port, &registry, &encode_document(&document).unwrap()).await;
            assert_eq!(gate_of(&receipt), ConsumerRefusal::Unauthenticated);
        }

        // A non-canonical outer frame, and a nested frame over the patch cap.
        let receipt = admit_document(&port, &registry(), &rmp_serde::to_vec(&good).unwrap()).await;
        assert!(matches!(
            gate_of(&receipt),
            ConsumerRefusal::NotCanonical | ConsumerRefusal::Malformed
        ));
        let mut oversize = good.clone();
        oversize.patch = vec![0u8; patch::MAX_PATCH_BYTES + 1];
        let receipt =
            admit_document(&port, &registry(), &encode_document(&oversize).unwrap()).await;
        assert_eq!(gate_of(&receipt), ConsumerRefusal::TooLarge);

        // Nothing above reached the kernel.
        assert_eq!(mailbox.snapshot().await.unwrap().revision, before.revision);
        // The port holds its own clone of the mailbox: the owner task ends
        // when the last sender does.
        drop(port);
        drop(mailbox);
        task.await.unwrap();
    }

    // ---- phase -------------------------------------------------------------

    #[test]
    fn a_component_only_active_consumer_patch_answers_nothing_and_commits() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, mirror) = mirror_kernel(directory.path(), "Component Only");
        let receipt = as_consumer(&mut kernel, operations(consumer_ops(&mirror, 2).operations))
            .expect("the ordinary batch commits");
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
    }

    #[test]
    fn a_declaring_active_consumer_patch_without_an_answer_is_answer_required() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, mirror) = mirror_kernel(directory.path(), "Answer Required");
        let error = as_consumer(
            &mut kernel,
            declaring(vec![Declaration::Fact(FactDeclaration {
                handle: DraftHandle::new("claim"),
                label: "The Shortfall".into(),
                statement: Statement::new("The hold reports a shortfall.").unwrap(),
                standing: FactStandingRef::Claimed {
                    by: Ref::Existing(mirror.first),
                },
            })]),
        )
        .unwrap_err();
        assert!(matches!(error, KernelError::AnswerRequired), "{error:?}");
    }

    #[test]
    fn a_consumer_answers_a_missing_structure_boundary_on_its_own_mirror() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, mirror) = mirror_kernel(directory.path(), "Boundary");
        let answered = missing_structure(&kernel, mirror.first);

        let answering = |answer: PatchAnswer| CommandBody::AdmitPatch {
            answers: Some(answer),
            patch: WorldPatch {
                declarations: Vec::new(),
                operations: clearing_grant(&mirror),
                evidence: Vec::new(),
            },
        };

        // A boundary about a foreign subject is not covered by a consumer's
        // ground: `require_patch_author` refuses it.
        let foreign = missing_structure(&kernel, mirror.local);
        let error =
            as_consumer(&mut kernel, answering(PatchAnswer::Boundary(foreign))).unwrap_err();
        assert!(matches!(error, KernelError::Unauthorized), "{error:?}");

        // A deficit is jurisdictional and a consumer holds no jurisdiction.
        // `ground_covers` would refuse it, but this fixture names no scale
        // intent, so `require_answer` — which runs first — refuses it as
        // underived. Either way the consumer never reaches the deficit lane.
        for jurisdiction in [
            JurisdictionKey::Uncovered,
            JurisdictionKey::PlaceSubtree(mirror.commons),
        ] {
            let error = as_consumer(&mut kernel, answering(PatchAnswer::Deficit(jurisdiction)))
                .unwrap_err();
            assert!(matches!(error, KernelError::AnswerNotDerived), "{error:?}");
            assert!(!ground_covers(
                &kernel.state,
                PatchGround::Consumer(consumer()),
                &PatchAnswer::Deficit(jurisdiction)
            ));
        }

        // A patch that does not satisfy the boundary it answered is refused.
        let unsatisfying = as_consumer(
            &mut kernel,
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Boundary(answered.clone())),
                patch: consumer_ops(&mirror, 1),
            },
        )
        .unwrap_err();
        assert!(
            matches!(unsatisfying, KernelError::AnswerNotSatisfied),
            "{unsatisfying:?}"
        );

        // And the honest answer commits, clearing exactly what it named.
        as_consumer(
            &mut kernel,
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Boundary(answered.clone())),
                patch: WorldPatch {
                    declarations: Vec::new(),
                    operations: clearing_grant(&mirror),
                    evidence: Vec::new(),
                },
            },
        )
        .expect("the consumer answers its own boundary");
        assert!(
            !derive_boundaries(&kernel.state)
                .unwrap()
                .contains(&answered)
        );
    }

    // ---- replay -------------------------------------------------------------

    #[test]
    fn consumer_admission_replays_to_the_same_state_digest() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, mirror) = mirror_kernel(directory.path(), "Replay");
        let world_id = kernel.snapshot().unwrap().world_id;
        as_consumer(&mut kernel, operations(consumer_ops(&mirror, 1).operations)).unwrap();
        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            CommandBody::AdvanceTime {
                minutes: super::super::TickMinutes::new(30).unwrap(),
            },
        );
        as_consumer(&mut kernel, operations(consumer_ops(&mirror, 2).operations)).unwrap();
        let accepted = kernel.snapshot().unwrap();
        drop(kernel);

        let replayed = WorldKernel::open(directory.path().join("world.cc"), world_id).unwrap();
        assert_eq!(replayed.snapshot().unwrap(), accepted);
        drop(replayed);

        // A store written under the previous state schema is refused rather
        // than migrated.
        let path = directory.path().join("world.cc");
        let mut bytes = std::fs::read(&path).unwrap();
        let live = super::super::STATE_SCHEMA.as_bytes();
        let previous = b"ghostlight.world_state.consumer.v0";
        assert_eq!(live.len(), previous.len());
        let mut rewritten = 0;
        for index in 0..bytes.len().saturating_sub(live.len()) {
            if &bytes[index..index + live.len()] == live {
                bytes[index..index + live.len()].copy_from_slice(previous);
                rewritten += 1;
            }
        }
        assert!(rewritten > 0, "the store names no state schema");
        std::fs::write(&path, bytes).unwrap();
        assert!(WorldKernel::open(&path, world_id).is_err());
    }

    // ---- Soul: falsification of the pass's own claims ----------------------

    /// A committed decision under the forgery, with a real event beneath it,
    /// and proof that the honest state — a mirror with no controller and no
    /// grant — passes the same shape check.
    #[test]
    fn soul_a_forged_decision_event_on_a_mirror_is_refused_at_replay() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, mirror) = mirror_kernel(directory.path(), "Acted");
        let snapshot = kernel.snapshot().unwrap();
        let opportunity = opportunity_for(&snapshot, mirror.local);
        let affordance = opportunity.affordance_ids[0];
        let controller = opportunity.controller_id;
        submit_as(
            &mut kernel,
            CallerId::Controller(controller),
            CommandBody::ExerciseDecision {
                opportunity,
                invocation: DecisionInvocation {
                    affordance,
                    bindings: Vec::new(),
                    proposed: Vec::new(),
                    speech: Some(Statement::new("The hold is counted.").unwrap()),
                },
            },
        )
        .expect("an ordinary subject acts");
        assert!(
            !kernel.state.events.is_empty(),
            "the fixture must commit a real event for this forgery to mean anything"
        );

        // The honest state passes: an externally controlled subject with no
        // controller id and no affordance grant is legal.
        super::super::journal::verify_state_shape(&kernel.state)
            .expect("a world holding a mirror is a well-shaped world");

        // Re-scoping the committed event onto the mirror is refused: the
        // mirror's assignment has no controller id to match.
        let mut forged = kernel.state.clone();
        let mut event = forged.events.first().cloned().expect("a committed event");
        event.scope = DecisionScope {
            subject_id: mirror.first,
        };
        forged.events = vec![event];
        assert!(
            super::super::journal::verify_state_shape(&forged).is_err(),
            "a forged history in which the mirror acted replayed"
        );
    }

    /// The pass-10 deficit assertion lands on `AnswerNotDerived` because its
    /// fixture names no scale intent, so the answer rule refuses before the
    /// ground rule is consulted. With an intent the deficit is real, the answer
    /// rule passes, and `ground_covers` is what refuses.
    #[test]
    fn soul_a_consumer_cannot_answer_a_derived_deficit() {
        let directory = tempfile::tempdir().unwrap();
        let mut creation = creation(CommandId::new(), "Deficit");
        creation.scale_intent = WorldScaleIntentRef {
            targets: BTreeMap::from([(SubjectKind::Person, 9)]),
            jurisdictions: BTreeMap::from([(DraftHandle::new(super::super::tests::COMMONS), 1000)]),
        };
        let mut kernel = WorldKernel::create(
            directory.path().join("world.cc"),
            creation,
            &auth_principal(owner()),
        )
        .expect("a created world")
        .0;
        let commons = entity_named(&kernel, "The Commons");
        let speak = speak_entry(&kernel);
        let before = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &before,
            CommandBody::AdmitPatch {
                answers: None,
                patch: mirror_patch(commons, speak),
            },
        );
        let mirror = Mirror {
            commons,
            grain: entity_named(&kernel, "Winter Grain"),
            first: subject_named(&kernel, "The Sunk Hold"),
            second: subject_named(&kernel, "The Deeper Hold"),
            local: subject_named(&kernel, "The Rhythm Hall"),
        };
        activate(&mut kernel);

        let jurisdiction = JurisdictionKey::PlaceSubtree(commons);
        assert!(
            derive_scale_deficit(&kernel.state)
                .unwrap()
                .iter()
                .any(|row| row.jurisdiction == jurisdiction && row.deficit > 0),
            "the fixture must derive a real deficit for this refusal to mean anything"
        );
        // The elaborator holding that jurisdiction is covered by the same row.
        assert!(ground_covers(
            &kernel.state,
            PatchGround::Jurisdiction(jurisdiction),
            &PatchAnswer::Deficit(jurisdiction)
        ));
        assert!(!ground_covers(
            &kernel.state,
            PatchGround::Consumer(consumer()),
            &PatchAnswer::Deficit(jurisdiction)
        ));

        let error = as_consumer(
            &mut kernel,
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Deficit(jurisdiction)),
                patch: WorldPatch {
                    declarations: Vec::new(),
                    operations: clearing_grant(&mirror),
                    evidence: Vec::new(),
                },
            },
        )
        .unwrap_err();
        assert!(matches!(error, KernelError::Unauthorized), "{error:?}");
    }

    /// The byte cap is ahead of serde, not beside it. Bytes msgpack cannot
    /// decode at all come back `TooLarge` over the cap and `Malformed` under
    /// it, which is only possible if the length gate runs first.
    #[test]
    fn soul_the_patch_byte_cap_bites_before_any_item_deserializes() {
        // 0xc1 is msgpack's never-used byte: nothing can deserialize from it.
        let over = vec![0xc1u8; patch::MAX_PATCH_BYTES + 1];
        assert_eq!(
            patch::decode_patch(&over),
            Err(PatchDecodeError::TooLarge {
                bytes: patch::MAX_PATCH_BYTES + 1
            })
        );
        let under = vec![0xc1u8; patch::MAX_PATCH_BYTES];
        assert_eq!(
            patch::decode_patch(&under),
            Err(PatchDecodeError::Malformed)
        );
    }

    /// Every refusal shape the ingress owns, each against the store's bytes.
    /// A refused document must move neither the revision nor one byte of the
    /// journal, and its receipt must name the gate that refused it.
    #[tokio::test]
    async fn soul_every_refused_document_leaves_the_store_byte_identical() {
        let directory = tempfile::tempdir().unwrap();
        let (mailbox, task, port, mirror, world_id) = mirror_mailbox(directory.path()).await;
        let path = directory.path().join("world.cc");
        let before = mailbox.snapshot().await.unwrap();
        assert!(before.revision > 0);
        // The store is open and locked by the owner task and its file is
        // preallocated, so neither its bytes nor its length reports growth while
        // it is live. Equality of state is read as the whole snapshot here, and
        // the committed rows are counted once the lock is gone.
        let good = consumer_ops(&mirror, 1);
        let document = |consumer: &str, secret: &str, key: &str, revision: u64| {
            build_document(world_id, consumer, secret, key, revision, None, &good).unwrap()
        };
        let over_cap = WorldPatch {
            declarations: (0..=patch::MAX_PATCH_DECLARATIONS)
                .map(|index| {
                    Declaration::Entity(EntityDeclaration {
                        handle: DraftHandle::new(&format!("shed-{index}")),
                        label: format!("Shed {index}"),
                        kind: EntityKind::Place,
                        container: None,
                    })
                })
                .collect(),
            operations: Vec::new(),
            evidence: Vec::new(),
        };
        let wrong_schema = {
            let mut wrong = document(CONSUMER, SECRET, "schema", before.revision);
            wrong.schema = "ghostlight.consumer_patch.v9".into();
            wrong
        };

        let cases: Vec<(&str, Vec<u8>, ConsumerRefusal)> = vec![
            (
                "a frame that is not msgpack at all",
                vec![0xc1u8; 32],
                ConsumerRefusal::Malformed,
            ),
            (
                "a non-canonical outer frame",
                rmp_serde::to_vec(&document(CONSUMER, SECRET, "compact", before.revision)).unwrap(),
                ConsumerRefusal::NotCanonical,
            ),
            (
                "an unknown schema string",
                encode_document(&wrong_schema).unwrap(),
                ConsumerRefusal::Schema,
            ),
            (
                "an unregistered consumer",
                encode_document(&document(OTHER, SECRET, "stranger", before.revision)).unwrap(),
                ConsumerRefusal::Unauthenticated,
            ),
            (
                "a wrong secret",
                encode_document(&document(
                    CONSUMER,
                    "not-the-secret",
                    "wrong",
                    before.revision,
                ))
                .unwrap(),
                ConsumerRefusal::Unauthenticated,
            ),
            (
                "a patch frame over the declaration cap",
                encode_document(
                    &build_document(
                        world_id,
                        CONSUMER,
                        SECRET,
                        "over-cap",
                        before.revision,
                        None,
                        &over_cap,
                    )
                    .unwrap(),
                )
                .unwrap(),
                ConsumerRefusal::TooManyDeclarations,
            ),
            (
                "a stale expected revision",
                encode_document(&document(CONSUMER, SECRET, "stale", before.revision - 1)).unwrap(),
                ConsumerRefusal::StaleRevision,
            ),
        ];

        for (name, bytes, expected) in cases {
            let receipt = admit_document(&port, &registry(), &bytes).await;
            let ConsumerOutcome::Refused { gate, .. } = &receipt.outcome else {
                panic!("{name} was not refused: {receipt:?}");
            };
            assert_eq!(*gate, expected, "{name}");
            if expected == ConsumerRefusal::StaleRevision {
                // The refusal carries the live revision, so the consumer knows
                // what to build against next.
                assert_eq!(receipt.revision, Some(before.revision), "{name}");
            }
            assert_eq!(
                mailbox.snapshot().await.unwrap(),
                before,
                "{name} moved the world"
            );
        }

        // And the same fixture still admits an honest document, so the loop
        // above proves refusal rather than a dead ingress.
        let honest =
            encode_document(&document(CONSUMER, SECRET, "honest", before.revision)).unwrap();
        let receipt = admit_document(&port, &registry(), &honest).await;
        assert!(
            matches!(receipt.outcome, ConsumerOutcome::Applied { .. }),
            "{receipt:?}"
        );
        assert_eq!(receipt.world_id, Some(world_id));
        assert!(receipt.command_id.is_some());
        assert_eq!(receipt.schema, CONSUMER_RECEIPT_SCHEMA);
        // The receipt decodes by its schema constant.
        let decoded: ConsumerReceiptDocument =
            rmp_serde::from_slice(&encode_receipt(&receipt).unwrap()).unwrap();
        assert_eq!(decoded, receipt);
        let applied = mailbox.snapshot().await.unwrap();
        assert_eq!(applied.revision, before.revision + 1);

        drop(port);
        drop(mailbox);
        task.await.unwrap();

        // Replay is what proves the store grew by exactly one row: a refusal
        // that had appended a commit would replay to a different revision and a
        // different world.
        let replayed = WorldKernel::open(&path, world_id).unwrap();
        assert_eq!(replayed.snapshot().unwrap(), applied);
        assert_eq!(replayed.snapshot().unwrap().revision, before.revision + 1);
    }

    /// The one derived-key recipe: deterministic, namespaced, and unambiguous
    /// across the three parts a consumer key is built from.
    #[test]
    fn soul_a_derived_command_key_is_deterministic_and_unambiguous() {
        let key = |namespace: &str, parts: &[&str]| CommandId::derived(namespace, parts);
        assert_eq!(key("ns", &["a", "b"]), key("ns", &["a", "b"]));
        // Length-prefixed parts: a split cannot be moved without changing the key.
        assert_ne!(key("ns", &["a", "b"]), key("ns", &["ab", ""]));
        assert_ne!(key("ns", &["a", "b"]), key("other", &["a", "b"]));

        let derived: Vec<CommandId> = [
            ("world-one", "consumer-one", "key-one"),
            ("world-one", "consumer-one", "key-two"),
            ("world-one", "consumer-two", "key-one"),
            ("world-two", "consumer-one", "key-one"),
        ]
        .iter()
        .map(|(world, consumer, idempotency)| {
            key(CONSUMER_COMMAND_NAMESPACE, &[world, consumer, idempotency])
        })
        .collect();
        for (index, one) in derived.iter().enumerate() {
            for other in &derived[index + 1..] {
                assert_ne!(one, other, "two derived command keys collided");
            }
        }
        assert_ne!(ConsumerId::of_name(CONSUMER), ConsumerId::of_name(OTHER));
        assert_eq!(ConsumerId::of_name(CONSUMER), ConsumerId::of_name(CONSUMER));
    }

    /// The mirror is not an actor at the second gate either: a real committed
    /// decision, re-scoped onto the mirror and pushed straight through
    /// `apply_effect`, is refused and leaves the candidate state untouched.
    #[test]
    fn soul_a_forged_exercise_naming_a_mirror_is_refused_at_apply_effect() {
        let directory = tempfile::tempdir().unwrap();
        let (kernel, mirror) = mirror_kernel(directory.path(), "Forged Act");
        let snapshot = kernel.snapshot().unwrap();
        let opportunity = opportunity_for(&snapshot, mirror.local);
        let affordance = opportunity.affordance_ids[0];
        let controller = opportunity.controller_id;
        let command_id = CommandId::new();
        let effect = super::super::reduce(
            &kernel.state,
            &command(
                &snapshot,
                command_id,
                CallerId::Controller(controller),
                CommandBody::ExerciseDecision {
                    opportunity: opportunity.clone(),
                    invocation: DecisionInvocation {
                        affordance,
                        bindings: Vec::new(),
                        proposed: Vec::new(),
                        speech: Some(Statement::new("The hold is counted.").unwrap()),
                    },
                },
            ),
        )
        .expect("the ordinary decision reduces");
        let super::super::WorldEffect::DecisionExercised {
            opportunity,
            invocation,
            mut event,
        } = effect
        else {
            panic!("an exercised decision produces one effect shape");
        };
        let scope = DecisionScope {
            subject_id: mirror.first,
        };
        event.scope = scope;
        let forged = super::super::WorldEffect::DecisionExercised {
            opportunity: DecisionOpportunity {
                scope,
                ..opportunity
            },
            invocation,
            event,
        };
        let mut candidate = kernel.state.clone();
        let error = super::super::apply_effect(
            &mut candidate,
            command_id,
            &CallerId::Controller(controller),
            &forged,
        )
        .unwrap_err();
        assert!(
            matches!(
                error,
                KernelError::Invariant(_)
                    | KernelError::OpportunityMismatch
                    | KernelError::ScopeChanged { .. }
            ),
            "{error:?}"
        );
        assert_eq!(candidate, kernel.state);
    }

    /// A mixed history — owner genesis, owner admission, an elaborator patch, a
    /// clock tick, and two consumer documents — replays from the store to the
    /// same snapshot, and the previous pass's state schema is refused rather
    /// than migrated.
    #[test]
    fn soul_a_mixed_history_replays_and_the_elaboration_schema_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let (mut kernel, mirror) = mirror_kernel(directory.path(), "Mixed");
        let world_id = kernel.snapshot().unwrap().world_id;

        as_consumer(&mut kernel, operations(consumer_ops(&mirror, 1).operations)).unwrap();

        // An elaborator writing inside the commons subtree, so all three
        // authors appear in one store.
        let boundary = missing_structure(&kernel, mirror.local);
        let elaborator = CallerId::System(SystemCapability::Elaborator {
            jurisdiction: JurisdictionKey::PlaceSubtree(mirror.commons),
        });
        submit_as(
            &mut kernel,
            elaborator,
            CommandBody::AdmitPatch {
                answers: Some(PatchAnswer::Boundary(boundary)),
                patch: WorldPatch {
                    declarations: Vec::new(),
                    operations: vec![ComponentOp::GrantAuthority {
                        holder: Ref::Existing(mirror.second),
                        grant: AuthorityGrantRef {
                            kind: AuthorityKindName(HOLD_KIND.into()),
                            over: AuthorityTargetRef::Subject(Ref::Existing(mirror.local)),
                        },
                    }],
                    evidence: Vec::new(),
                },
            },
        )
        .expect("the elaborator answers a boundary inside its jurisdiction");

        let snapshot = kernel.snapshot().unwrap();
        submit_owner(
            &mut kernel,
            &snapshot,
            CommandBody::AdvanceTime {
                minutes: super::super::TickMinutes::new(30).unwrap(),
            },
        );
        as_consumer(&mut kernel, operations(consumer_ops(&mirror, 2).operations)).unwrap();

        let accepted = kernel.snapshot().unwrap();
        let digest = kernel.state.clone();
        drop(kernel);
        let replayed = WorldKernel::open(directory.path().join("world.cc"), world_id).unwrap();
        assert_eq!(replayed.snapshot().unwrap(), accepted);
        assert_eq!(replayed.state, digest);

        // The pass-9 schema string, named exactly, is refused by the same shape
        // check every replayed store passes.
        let mut previous = replayed.state.clone();
        previous.schema = "ghostlight.world_state.elaboration.v1".into();
        assert!(super::super::journal::verify_state_shape(&previous).is_err());
        assert_eq!(
            super::super::STATE_SCHEMA,
            "ghostlight.world_state.consumer.v1"
        );
    }

    /// The registry holds digests, and nothing it can say about a bad
    /// credentials file can carry one.
    #[test]
    fn soul_no_secret_reaches_a_registry_error_or_a_registry_debug() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("consumers.cfg");

        // A missing file is no consumers, not an open door and not a panic.
        let missing = ConsumerRegistry::from_secret_file(&path).expect("a missing file is empty");
        assert!(missing.authenticate(CONSUMER, SECRET).is_none());

        // A file holding a plaintext secret where a digest belongs is an error
        // that does not echo the secret.
        std::fs::write(&path, format!("{CONSUMER} = {SECRET}\n")).unwrap();
        let error = ConsumerRegistry::from_secret_file(&path).unwrap_err();
        assert!(!error.contains(SECRET), "{error}");

        // The honest file authenticates, and the registry's own rendering
        // carries only the digest.
        let digest = Sha256::digest(SECRET.as_bytes());
        let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        std::fs::write(&path, format!("# consumers\n\n{CONSUMER} = {hex}\n")).unwrap();
        let registry = ConsumerRegistry::from_secret_file(&path).expect("a well-formed file");
        assert_eq!(
            registry.authenticate(CONSUMER, SECRET),
            Some(ConsumerId::of_name(CONSUMER))
        );
        assert!(registry.authenticate(CONSUMER, "not-the-secret").is_none());
        assert!(registry.authenticate(OTHER, SECRET).is_none());
        assert!(!format!("{registry:?}").contains(SECRET));
    }

    /// The registry's own `Debug` carries only digests to begin with, but the
    /// document that arrives off the wire carries the secret in plaintext, and
    /// its `derive(Debug)` would have printed it. This is the document-side
    /// sibling of `soul_no_secret_reaches_a_registry_error_or_a_registry_debug`.
    #[test]
    fn soul_no_secret_reaches_a_consumer_patch_document_debug() {
        let empty = WorldPatch {
            declarations: Vec::new(),
            operations: Vec::new(),
            evidence: Vec::new(),
        };
        let document = build_document(
            WorldId::nil_for_test(),
            CONSUMER,
            SECRET,
            "idempotency-key",
            1,
            None,
            &empty,
        )
        .unwrap();
        let rendered = format!("{document:?}");
        assert!(!rendered.contains(SECRET), "{rendered}");
        assert!(rendered.contains("<redacted>"), "{rendered}");
    }
}
