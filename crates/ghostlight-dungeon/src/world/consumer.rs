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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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
        PatchDecodeError::TooLarge { .. }
        | PatchDecodeError::TooManyDeclarations { .. }
        | PatchDecodeError::TooManyOperations { .. }
        | PatchDecodeError::TooManyEvidence { .. } => ConsumerRefusal::TooLarge,
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
