//! The world's authoring loop, inside the world.
//!
//! One jurisdiction's elaborator selects a derived answer, retrieves evidence
//! for the referents that answer names, drives a tool conversation over the
//! reducer's own vocabulary, accumulates a draft patch, submits it, and repairs
//! the same draft from the complete mismatch set the kernel returns.
//!
//! It owns no world truth. It holds no authority beyond the capability the
//! mailbox mints for it, it clears no boundary — boundaries are derived, and a
//! commit that makes the predicate stop holding is the only thing that ends one
//! — and it writes nothing but `AdmitPatch`.

use super::controllers::{
    ControllerError, ControllerNeed, ControllerWork, ControllerWorkLookup, ControllerWorkStore,
    ControllerWorkWrite, InferenceEvent, InferenceOutput, InferencePort, InferencePurpose,
    InferenceRequest, PreparedInference, canonical_model, prepared_matches_request,
    tool_decode_need, tool_request,
};
#[cfg(test)]
use super::patch::RECORD_GAP_PATCH_TOOL;
use super::patch::{ComponentOp, FactStandingRef};
use super::patch::{
    PATCH_TOOLS, PatchToolShape, SUBMIT_PATCH_TOOL, patch_tool_signatures, patch_tools,
};
use super::{
    BoundaryDigest, CausalBoundary, CommandId, Declaration, ElaborationPort, EntityId, EvidenceRef,
    JurisdictionKey, KernelError, MailboxError, Mismatch, PatchAnswer, SubjectId, WorldId,
    WorldPatch, WorldPhase, WorldSnapshot,
};
use async_trait::async_trait;
use codex_connector::CodexInputItem;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// The elaborator wants many tool calls inside one round and several rounds for
/// repair, where the operational agent wants one terminal choice.
/// `TOOL_STEP_BUDGET` is left alone for the lane it belongs to.
const ELABORATION_ROUND_BUDGET: usize = 6;

/// The author owns the size of what it authors. The kernel gets no cap:
/// `resolve_patch` is total, and a cap there would be a second authority over
/// what a patch may be.
const MAX_DRAFT_DECLARATIONS: usize = 64;
const MAX_DRAFT_OPERATIONS: usize = 128;

const ELABORATION_NAMESPACE: &str = "ghostlight.command.elaboration.v1";

const ELABORATION_INSTRUCTIONS: &str = "Use only the supplied tools to author structure inside your jurisdiction. Answer the boundary or deficit you were given, then submit. Recording a gap changes nothing.";

/// The answer a session is bound to, plus the ancestry it was built against.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ElaboratorSession {
    pub(super) world_id: WorldId,
    pub(super) jurisdiction: JurisdictionKey,
    pub(super) answer: PatchAnswer,
    /// The boundary's own digest, or for a deficit the digest of
    /// `(world_id, jurisdiction, "deficit", ancestry)`.
    pub(super) answer_digest: BoundaryDigest,
    /// The commit digest the draft is built against.
    pub(super) ancestry: String,
}

/// The stages one authoring session moves through. The draft is **not** a
/// field: it is re-derived from `completed` by `evaluate_elaboration_loop`,
/// exactly as an operational capture is, so a resumed session cannot submit a
/// draft the conversation does not produce. `last_mismatches` is stored,
/// because it is the one input that cannot be re-derived from `completed` — it
/// came from the kernel, not the model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "stage", rename_all = "snake_case")]
pub(super) enum ElaborationCheckpoint {
    ElaboratorInFlight {
        command_id: CommandId,
        session: ElaboratorSession,
        agent_prompt: String,
        last_mismatches: Vec<Mismatch>,
        completed: Vec<InferenceOutput>,
        invocation: PreparedInference,
    },
    ReadyToSubmit {
        command_id: CommandId,
        session: ElaboratorSession,
        agent_prompt: String,
        last_mismatches: Vec<Mismatch>,
        completed: Vec<InferenceOutput>,
    },
    /// The round budget ran out, or the model finished without a submit. The
    /// boundary stays derived and visible; nothing is repaired by waiting.
    NoPatch {
        command_id: CommandId,
        session: ElaboratorSession,
        agent_prompt: String,
        completed: Vec<InferenceOutput>,
        gaps: Vec<ControllerNeed>,
    },
}

impl ElaborationCheckpoint {
    pub(super) fn command_id(&self) -> CommandId {
        match self {
            Self::ElaboratorInFlight { command_id, .. }
            | Self::ReadyToSubmit { command_id, .. }
            | Self::NoPatch { command_id, .. } => *command_id,
        }
    }

    fn session(&self) -> &ElaboratorSession {
        match self {
            Self::ElaboratorInFlight { session, .. }
            | Self::ReadyToSubmit { session, .. }
            | Self::NoPatch { session, .. } => session,
        }
    }

    pub(super) fn integrity_is_valid(&self) -> bool {
        match self {
            Self::ElaboratorInFlight {
                command_id,
                agent_prompt,
                last_mismatches,
                completed,
                invocation,
                ..
            } => {
                !agent_prompt.is_empty()
                    && canonical_model(&invocation.invocation.request.model)
                    && match evaluate_elaboration_loop(agent_prompt, last_mismatches, completed) {
                        Ok(ElaborationLoopEvaluation::Continue { conversation }) => {
                            elaboration_request(
                                *command_id,
                                completed.len(),
                                &invocation.invocation.request.model,
                                conversation,
                            )
                            .is_ok_and(|expected| {
                                prepared_matches_request(
                                    invocation,
                                    &expected,
                                    *command_id,
                                    completed.len(),
                                )
                            })
                        }
                        Ok(ElaborationLoopEvaluation::Complete { .. }) | Err(_) => false,
                    }
            }
            Self::ReadyToSubmit {
                agent_prompt,
                last_mismatches,
                completed,
                ..
            } => derive_elaboration_capture(agent_prompt, last_mismatches, completed)
                .is_ok_and(|capture| capture.submitted),
            Self::NoPatch {
                agent_prompt,
                completed,
                ..
            } => derive_elaboration_capture(agent_prompt, &[], completed)
                .is_ok_and(|capture| !capture.submitted),
        }
    }
}

/// A session may gather more evidence and may end, but it may never rewrite the
/// answer it bound to.
pub(super) fn valid_elaboration_progression(
    existing: &ElaborationCheckpoint,
    next: &ElaborationCheckpoint,
) -> bool {
    if existing.command_id() != next.command_id() || existing.session() != next.session() {
        return false;
    }
    match (existing, next) {
        (
            ElaborationCheckpoint::ElaboratorInFlight {
                completed: existing,
                ..
            },
            ElaborationCheckpoint::ElaboratorInFlight {
                completed: next, ..
            }
            | ElaborationCheckpoint::ReadyToSubmit {
                completed: next, ..
            }
            | ElaborationCheckpoint::NoPatch {
                completed: next, ..
            },
        ) => next.len() >= existing.len() && next.starts_with(existing),
        // A rejection reopens the same command id for repair. A rejected
        // command mutates nothing, so the round evidence starts over.
        (
            ElaborationCheckpoint::ReadyToSubmit { .. },
            ElaborationCheckpoint::ElaboratorInFlight {
                last_mismatches, ..
            },
        ) => !last_mismatches.is_empty(),
        _ => false,
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{detail}")]
pub(super) struct EvidenceError {
    detail: String,
}

/// The referents the answer names, as world labels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct EvidenceQuery {
    pub(super) referents: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct EvidenceReceipt {
    /// The exact reference an admitted patch may carry.
    pub(super) reference: EvidenceRef,
    /// What the elaborator is allowed to read. Prompt material, never state.
    pub(super) excerpt: String,
    pub(super) source: String,
}

/// The Vault seam. Provenance has an owner and it is the runner: a citation
/// outside the round's retrieved set never reaches the kernel. The reducer does
/// not and must not check that a Vault receipt exists; it checks that the
/// string is canonical and that an `Admit`'s reference is listed in the same
/// patch.
#[async_trait]
pub(super) trait EvidenceSource: Send + Sync {
    async fn retrieve(&self, query: &EvidenceQuery) -> Result<Vec<EvidenceReceipt>, EvidenceError>;
}

/// What `runtime.rs` supplies until a retrieval organ lands. The bound is real
/// and stated rather than hidden: with no receipts an elaborator can declare
/// structure and operate on it, but cannot mint quantity, because `admit`
/// requires a listed reference and there are none. Creation of quantity without
/// provenance should be impossible, not merely discouraged.
pub(super) struct NullEvidenceSource;

#[async_trait]
impl EvidenceSource for NullEvidenceSource {
    async fn retrieve(
        &self,
        _query: &EvidenceQuery,
    ) -> Result<Vec<EvidenceReceipt>, EvidenceError> {
        Ok(Vec::new())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ElaborationCapture {
    pub(super) draft: WorldPatch,
    pub(super) gaps: Vec<ControllerNeed>,
    pub(super) inference_receipts: Vec<String>,
    pub(super) submitted: bool,
}

pub(super) enum ElaborationLoopEvaluation {
    Continue { conversation: Vec<CodexInputItem> },
    Complete { capture: ElaborationCapture },
}

/// What one `step` did, for the driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ElaborationOutcome {
    /// No boundary and no deficit in this jurisdiction: the terminating
    /// condition.
    Clean,
    /// The world is not Active, so there is nothing to elaborate: seed
    /// admission is the owner's lane.
    Inactive,
    Committed,
    /// The kernel returned a complete mismatch set; the next round repairs it.
    Rejected,
    /// The world moved under the session. Re-select; this is not a retry loop.
    Superseded,
    /// The round budget was spent without a submit. The boundary stays derived.
    NoPatch,
}

/// One jurisdiction's authoring loop. Holds no state between steps; every field
/// is a port or an identity.
pub(crate) struct ElaborationRunner {
    mailbox: ElaborationPort,
    inference: Arc<dyn InferencePort>,
    evidence: Arc<dyn EvidenceSource>,
    work: Arc<dyn ControllerWorkStore>,
    model: String,
}

impl ElaborationRunner {
    pub(super) fn new(
        mailbox: ElaborationPort,
        inference: Arc<dyn InferencePort>,
        evidence: Arc<dyn EvidenceSource>,
        work: Arc<dyn ControllerWorkStore>,
        model: String,
    ) -> Self {
        Self {
            mailbox,
            inference,
            evidence,
            work,
            model,
        }
    }

    /// One answer, start to finish. Session identity is derived, so resumption
    /// needs no registry: a boundary that survives a repair keeps its digest and
    /// therefore its command id, and a boundary whose digest moved is a
    /// different answer with a different id.
    pub(super) async fn step(
        &self,
        jurisdiction: JurisdictionKey,
    ) -> Result<ElaborationOutcome, ControllerError> {
        let snapshot = self.mailbox.snapshot().await.map_err(snapshot_error)?;
        if snapshot.phase != WorldPhase::Active {
            return Ok(ElaborationOutcome::Inactive);
        }
        let Some(session) = select_answer(&snapshot, jurisdiction)? else {
            return Ok(ElaborationOutcome::Clean);
        };
        let command_id = session_command_id(&session)?;

        let receipts = self
            .evidence
            .retrieve(&EvidenceQuery {
                referents: answer_referents(&snapshot, &session.answer),
            })
            .await
            .map_err(|error| ControllerError::WorkPersistence(error.to_string()))?;
        let allowed: BTreeSet<EvidenceRef> = receipts
            .iter()
            .map(|receipt| receipt.reference.clone())
            .collect();

        let existing = match self.work.lookup(command_id).await? {
            ControllerWorkLookup::Missing => None,
            ControllerWorkLookup::Confirmed(ControllerWork::Elaboration(checkpoint))
            | ControllerWorkLookup::CustodyUncertain(ControllerWork::Elaboration(checkpoint)) => {
                if checkpoint.session() != &session {
                    return Ok(ElaborationOutcome::Superseded);
                }
                Some(checkpoint)
            }
            ControllerWorkLookup::Confirmed(_) | ControllerWorkLookup::CustodyUncertain(_) => {
                return Err(ControllerError::CommandMismatch);
            }
        };

        let (agent_prompt, mut last_mismatches, mut completed) = match existing {
            Some(ElaborationCheckpoint::NoPatch { .. }) => return Ok(ElaborationOutcome::NoPatch),
            Some(
                ElaborationCheckpoint::ElaboratorInFlight {
                    agent_prompt,
                    last_mismatches,
                    completed,
                    ..
                }
                | ElaborationCheckpoint::ReadyToSubmit {
                    agent_prompt,
                    last_mismatches,
                    completed,
                    ..
                },
            ) => (agent_prompt, last_mismatches, completed),
            None => (
                build_prompt(&snapshot, &session, &receipts, &[]),
                Vec::new(),
                Vec::new(),
            ),
        };

        loop {
            match evaluate_elaboration_loop(&agent_prompt, &last_mismatches, &completed)? {
                ElaborationLoopEvaluation::Continue { conversation } => {
                    let request = elaboration_request(
                        command_id,
                        completed.len(),
                        &self.model,
                        conversation,
                    )?;
                    let invocation = self.inference.prepare(request).map_err(|source| {
                        ControllerError::Inference {
                            purpose: InferencePurpose::Elaboration,
                            source,
                        }
                    })?;
                    self.persist(ElaborationCheckpoint::ElaboratorInFlight {
                        command_id,
                        session: session.clone(),
                        agent_prompt: agent_prompt.clone(),
                        last_mismatches: last_mismatches.clone(),
                        completed: completed.clone(),
                        invocation: invocation.clone(),
                    })
                    .await?;
                    let output =
                        self.inference
                            .infer(invocation.clone())
                            .await
                            .map_err(|source| ControllerError::Inference {
                                purpose: InferencePurpose::Elaboration,
                                source,
                            })?;
                    completed.push(output);
                }
                ElaborationLoopEvaluation::Complete { capture } => {
                    if !capture.submitted {
                        self.persist(ElaborationCheckpoint::NoPatch {
                            command_id,
                            session,
                            agent_prompt,
                            completed,
                            gaps: capture.gaps,
                        })
                        .await?;
                        return Ok(ElaborationOutcome::NoPatch);
                    }
                    self.persist(ElaborationCheckpoint::ReadyToSubmit {
                        command_id,
                        session: session.clone(),
                        agent_prompt: agent_prompt.clone(),
                        last_mismatches: last_mismatches.clone(),
                        completed: completed.clone(),
                    })
                    .await?;
                    let draft = filter_evidence(capture.draft, &allowed);
                    let submitted = self
                        .mailbox
                        .submit_elaboration(
                            command_id,
                            session.jurisdiction,
                            session.answer.clone(),
                            draft,
                        )
                        .await;
                    return match submitted {
                        Ok(_) => Ok(ElaborationOutcome::Committed),
                        Err(MailboxError::Kernel(KernelError::PatchRejected(mismatches))) => {
                            // A rejected command mutates nothing, so reusing the
                            // id is correct and the idempotency probe never sees
                            // a conflicting commit.
                            last_mismatches = mismatches;
                            let repaired =
                                build_prompt(&snapshot, &session, &receipts, &last_mismatches);
                            self.persist_repair(command_id, &session, &repaired, &last_mismatches)
                                .await?;
                            Ok(ElaborationOutcome::Rejected)
                        }
                        Err(MailboxError::Kernel(KernelError::AnswerNotDerived)) => {
                            Ok(ElaborationOutcome::Superseded)
                        }
                        Err(error) => Err(snapshot_error(error)),
                    };
                }
            }
        }
    }

    async fn persist(&self, checkpoint: ElaborationCheckpoint) -> Result<(), ControllerError> {
        match self
            .work
            .persist(&ControllerWork::Elaboration(checkpoint))
            .await?
        {
            ControllerWorkWrite::Applied | ControllerWorkWrite::AlreadyPresent => Ok(()),
            ControllerWorkWrite::CustodyUncertain => Err(ControllerError::WorkPersistence(
                "controller work custody is uncertain".into(),
            )),
        }
    }

    /// The rejection the next round must repair, persisted against the same
    /// command id and the same session.
    async fn persist_repair(
        &self,
        command_id: CommandId,
        session: &ElaboratorSession,
        agent_prompt: &str,
        last_mismatches: &[Mismatch],
    ) -> Result<(), ControllerError> {
        let conversation = match evaluate_elaboration_loop(agent_prompt, last_mismatches, &[])? {
            ElaborationLoopEvaluation::Continue { conversation } => conversation,
            ElaborationLoopEvaluation::Complete { .. } => {
                return Err(ControllerError::Serialization(
                    "a repair round completed before any evidence".into(),
                ));
            }
        };
        let request = elaboration_request(command_id, 0, &self.model, conversation)?;
        let invocation =
            self.inference
                .prepare(request)
                .map_err(|source| ControllerError::Inference {
                    purpose: InferencePurpose::Elaboration,
                    source,
                })?;
        self.persist(ElaborationCheckpoint::ElaboratorInFlight {
            command_id,
            session: session.clone(),
            agent_prompt: agent_prompt.to_owned(),
            last_mismatches: last_mismatches.to_vec(),
            completed: Vec::new(),
            invocation,
        })
        .await
    }

    /// One sweep over every jurisdiction the world's scale intent names, then
    /// the uncovered residual. Sequential: a boundary binds to its own digest,
    /// so the loops' logical independence is preserved without eight tasks
    /// against one connector and a capacity-32 mailbox.
    pub(crate) async fn sweep(&self) -> Result<(), ControllerError> {
        let snapshot = self.mailbox.snapshot().await.map_err(snapshot_error)?;
        let mut jurisdictions: Vec<JurisdictionKey> = snapshot
            .scale_deficit
            .iter()
            .map(|row| row.jurisdiction)
            .collect();
        jurisdictions.dedup();
        if !jurisdictions.contains(&JurisdictionKey::Uncovered) {
            jurisdictions.push(JurisdictionKey::Uncovered);
        }
        for jurisdiction in jurisdictions {
            // The stop condition is "no open boundary and no deficit", plus the
            // only other fixed point: a pass over this answer that admits
            // nothing. Without it a boundary the model cannot answer becomes a
            // hot spin against a paid endpoint.
            match self.step(jurisdiction).await {
                Ok(_) => {}
                Err(error) if error.requires_quarantine() => return Err(error),
                Err(error) => {
                    tracing::debug!(%error, "elaboration step did not admit a patch");
                }
            }
        }
        Ok(())
    }
}

fn snapshot_error(error: MailboxError) -> ControllerError {
    ControllerError::Snapshot(error)
}

/// The oldest derived boundary in this jurisdiction, else its first nonzero
/// deficit row, else nothing. `snapshot.boundaries` is already ordered by the
/// kernel's derivation, so "oldest" is "first in that order".
fn select_answer(
    snapshot: &WorldSnapshot,
    jurisdiction: JurisdictionKey,
) -> Result<Option<ElaboratorSession>, ControllerError> {
    let ancestry = snapshot.last_commit_digest.clone().unwrap_or_default();
    if let Some(boundary) = snapshot
        .boundaries
        .iter()
        .find(|boundary| boundary_in(snapshot, jurisdiction, boundary))
    {
        return Ok(Some(ElaboratorSession {
            world_id: snapshot.world_id,
            jurisdiction,
            answer: PatchAnswer::Boundary(boundary.clone()),
            answer_digest: boundary_digest(boundary),
            ancestry,
        }));
    }
    let deficit = snapshot
        .scale_deficit
        .iter()
        .any(|row| row.jurisdiction == jurisdiction && row.deficit > 0);
    if !deficit {
        return Ok(None);
    }
    // A deficit mixes the ancestry, so each admitted commit opens a fresh
    // session and the loop advances instead of resubmitting one id forever.
    let digest = digest_of(&(snapshot.world_id, jurisdiction, "deficit", &ancestry))?;
    Ok(Some(ElaboratorSession {
        world_id: snapshot.world_id,
        jurisdiction,
        answer: PatchAnswer::Deficit(jurisdiction),
        answer_digest: BoundaryDigest::from_digest(digest),
        ancestry,
    }))
}

/// Selection reads the same covering the kernel's authority check does: a
/// boundary in a nested child is inside its parent's jurisdiction.
fn boundary_in(
    snapshot: &WorldSnapshot,
    jurisdiction: JurisdictionKey,
    boundary: &CausalBoundary,
) -> bool {
    let subject_place = |subject: SubjectId| {
        snapshot
            .subjects
            .iter()
            .find(|row| row.id == subject)
            .and_then(|row| row.position)
    };
    match jurisdiction {
        JurisdictionKey::Uncovered => match boundary {
            CausalBoundary::UnelaboratedDestination { .. } => false,
            CausalBoundary::MissingStructure { subject, .. }
            | CausalBoundary::PolityInCausalRange { subject, .. }
            | CausalBoundary::IndividuationRequired {
                population: subject,
                ..
            } => subject_place(*subject).is_none(),
        },
        JurisdictionKey::PlaceSubtree(root) => match boundary {
            CausalBoundary::UnelaboratedDestination { place, .. } => {
                snapshot_covers_place(snapshot, root, *place)
            }
            CausalBoundary::MissingStructure { subject, .. }
            | CausalBoundary::PolityInCausalRange { subject, .. }
            | CausalBoundary::IndividuationRequired {
                population: subject,
                ..
            } => subject_place(*subject)
                .is_some_and(|place| snapshot_covers_place(snapshot, root, place)),
        },
    }
}

fn snapshot_covers_place(snapshot: &WorldSnapshot, root: EntityId, place: EntityId) -> bool {
    let containers: BTreeMap<EntityId, Option<EntityId>> = snapshot
        .places
        .iter()
        .map(|entry| (entry.id, entry.container))
        .collect();
    let mut current = Some(place);
    for _ in 0..=containers.len() {
        match current {
            None => return false,
            Some(node) if node == root => return true,
            Some(node) => current = containers.get(&node).copied().flatten(),
        }
    }
    false
}

fn boundary_digest(boundary: &CausalBoundary) -> BoundaryDigest {
    match boundary {
        CausalBoundary::UnelaboratedDestination { scope, .. }
        | CausalBoundary::MissingStructure { scope, .. }
        | CausalBoundary::PolityInCausalRange { scope, .. }
        | CausalBoundary::IndividuationRequired { scope, .. } => scope.clone(),
    }
}

/// The world labels the answer names, which is what the Vault is asked for.
fn answer_referents(snapshot: &WorldSnapshot, answer: &PatchAnswer) -> Vec<String> {
    let place_label = |id: EntityId| {
        snapshot
            .places
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.label.clone())
    };
    let subject_label = |id: SubjectId| {
        snapshot
            .subjects
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.label.clone())
    };
    match answer {
        PatchAnswer::Boundary(CausalBoundary::UnelaboratedDestination { route, place, .. }) => {
            let mut referents: Vec<String> = place_label(*place).into_iter().collect();
            referents.extend(
                snapshot
                    .routes
                    .iter()
                    .find(|entry| entry.id == *route)
                    .map(|entry| entry.label.clone()),
            );
            referents
        }
        PatchAnswer::Boundary(
            CausalBoundary::MissingStructure { subject, .. }
            | CausalBoundary::PolityInCausalRange { subject, .. }
            | CausalBoundary::IndividuationRequired {
                population: subject,
                ..
            },
        ) => subject_label(*subject).into_iter().collect(),
        PatchAnswer::Deficit(JurisdictionKey::PlaceSubtree(root)) => {
            place_label(*root).into_iter().collect()
        }
        PatchAnswer::Deficit(JurisdictionKey::Uncovered) => Vec::new(),
    }
}

fn build_prompt(
    snapshot: &WorldSnapshot,
    session: &ElaboratorSession,
    receipts: &[EvidenceReceipt],
    last_mismatches: &[Mismatch],
) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are elaborating one jurisdiction of a world.\n\n");
    prompt.push_str(&format!("World: {}\n", snapshot.title));
    prompt.push_str(&format!(
        "Jurisdiction: {}\n",
        render_jurisdiction(snapshot, session.jurisdiction)
    ));
    prompt.push_str(&format!(
        "Answer: {}\n\n",
        render_answer(snapshot, &session.answer)
    ));
    if receipts.is_empty() {
        prompt.push_str(
            "No evidence receipts were retrieved. You may declare structure and operate on it; you cannot admit quantity without a receipt.\n\n",
        );
    } else {
        prompt.push_str("Evidence you may cite, exactly as written:\n");
        for receipt in receipts {
            prompt.push_str(&format!(
                "- {} ({}): {}\n",
                receipt.reference.text(),
                receipt.source,
                receipt.excerpt
            ));
        }
        prompt.push('\n');
    }
    if !last_mismatches.is_empty() {
        prompt.push_str("Your previous patch was refused. Repair every site:\n");
        for mismatch in last_mismatches {
            prompt.push_str(&format!("- {}\n", render_mismatch(mismatch)));
        }
        prompt.push('\n');
    }
    prompt.push_str(&format!("Tools: {}\n", patch_tool_signatures()));
    prompt
}

fn render_jurisdiction(snapshot: &WorldSnapshot, jurisdiction: JurisdictionKey) -> String {
    match jurisdiction {
        JurisdictionKey::PlaceSubtree(root) => snapshot
            .places
            .iter()
            .find(|entry| entry.id == root)
            .map_or_else(
                || "an unnamed place subtree".to_owned(),
                |entry| entry.label.clone(),
            ),
        JurisdictionKey::Uncovered => "everything no declared root covers".to_owned(),
    }
}

fn render_answer(snapshot: &WorldSnapshot, answer: &PatchAnswer) -> String {
    let referents = answer_referents(snapshot, answer).join(", ");
    match answer {
        PatchAnswer::Boundary(CausalBoundary::UnelaboratedDestination { .. }) => {
            format!("a route leads to a place with nothing in it: {referents}")
        }
        PatchAnswer::Boundary(CausalBoundary::MissingStructure { .. }) => format!(
            "a promise has a counterparty who can neither command nor litigate it: {referents}"
        ),
        PatchAnswer::Boundary(CausalBoundary::PolityInCausalRange { .. }) => {
            format!("a polity stands in causal range and is not modelled: {referents}")
        }
        PatchAnswer::Boundary(CausalBoundary::IndividuationRequired { .. }) => {
            format!("a population must be individuated: {referents}")
        }
        PatchAnswer::Deficit(_) => {
            "this jurisdiction holds fewer qualified subjects than the world means it to".to_owned()
        }
    }
}

fn render_mismatch(mismatch: &Mismatch) -> String {
    serde_json::to_string(mismatch)
        .unwrap_or_else(|_| "a structural check failed and could not be rendered".to_owned())
}

/// The same shape as the operational evaluator with three substitutions and no
/// fourth: a draft accumulates where the utterance did, many non-terminal calls
/// precede one terminal `submit`, and `record_gap` is a free detail string.
/// Serde is the only decoder; a decode failure becomes a gap, never a kernel
/// round trip.
pub(super) fn evaluate_elaboration_loop(
    prompt: &str,
    last_mismatches: &[Mismatch],
    completed: &[InferenceOutput],
) -> Result<ElaborationLoopEvaluation, ControllerError> {
    // The repair set is prompt material, folded in by `build_prompt`; it is
    // named here so a checkpoint that carries one cannot be re-derived without
    // it.
    let _ = last_mismatches;
    let mut conversation = vec![CodexInputItem::UserText {
        text: prompt.to_owned(),
    }];
    let mut draft = WorldPatch {
        declarations: Vec::new(),
        operations: Vec::new(),
        evidence: Vec::new(),
    };
    let mut gaps: Vec<ControllerNeed> = Vec::new();
    let mut receipts = Vec::new();
    let mut submitted = false;

    for (round, output) in completed.iter().enumerate() {
        if output.receipt_digest.is_empty() || output.receipt_digest.trim() != output.receipt_digest
        {
            return Err(ControllerError::ProviderContract {
                purpose: InferencePurpose::Elaboration,
                detail: "provider output has no canonical receipt digest".into(),
            });
        }
        receipts.push(output.receipt_digest.clone());
        let mut called_tool = false;
        for event in &output.events {
            match event {
                InferenceEvent::Text(text) => {
                    if !text.is_empty() {
                        conversation.push(CodexInputItem::AssistantText { text: text.clone() });
                    }
                }
                InferenceEvent::ToolCall {
                    call_id,
                    name,
                    arguments,
                } => {
                    called_tool = true;
                    conversation.push(CodexInputItem::ToolCall {
                        call_id: call_id.clone(),
                        name: name.clone(),
                        arguments: arguments.clone(),
                    });
                    let result =
                        apply_tool_call(name, arguments, &mut draft, &mut gaps, &mut submitted);
                    conversation.push(CodexInputItem::ToolResult {
                        call_id: call_id.clone(),
                        output: result,
                    });
                }
            }
        }

        let is_complete = submitted || !called_tool || round + 1 == ELABORATION_ROUND_BUDGET;
        if is_complete {
            if round + 1 != completed.len() {
                return Err(ControllerError::Serialization(
                    "elaboration evidence continued after total finalization".into(),
                ));
            }
            if !submitted && called_tool && round + 1 == ELABORATION_ROUND_BUDGET {
                gaps.push(ControllerNeed {
                    detail: "The elaboration round budget ended before a submit.".into(),
                });
            }
            draft.evidence.sort();
            draft.evidence.dedup();
            return Ok(ElaborationLoopEvaluation::Complete {
                capture: ElaborationCapture {
                    draft,
                    gaps,
                    inference_receipts: receipts,
                    submitted,
                },
            });
        }
    }

    if completed.len() >= ELABORATION_ROUND_BUDGET {
        return Err(ControllerError::Serialization(
            "elaboration evidence exceeded its round budget".into(),
        ));
    }
    Ok(ElaborationLoopEvaluation::Continue { conversation })
}

fn apply_tool_call(
    name: &str,
    arguments: &str,
    draft: &mut WorldPatch,
    gaps: &mut Vec<ControllerNeed>,
    submitted: &mut bool,
) -> String {
    let Some(entry) = PATCH_TOOLS.iter().find(|entry| entry.name == name) else {
        gaps.push(tool_decode_need(
            name,
            arguments,
            "tool is not in the patch catalog",
        ));
        return "unavailable tool recorded as a gap".into();
    };
    let Ok(Value::Object(fields)) = serde_json::from_str::<Value>(arguments) else {
        gaps.push(tool_decode_need(
            name,
            arguments,
            "arguments are not a JSON object",
        ));
        return "arguments recorded as a gap".into();
    };
    match entry.shape {
        PatchToolShape::Session if name == SUBMIT_PATCH_TOOL => {
            if *submitted {
                gaps.push(ControllerNeed {
                    detail: "The elaborator submitted twice for one answer.".into(),
                });
                return "a submit is already captured".into();
            }
            *submitted = true;
            "patch submitted".into()
        }
        PatchToolShape::Session => match fields.get("detail").and_then(Value::as_str) {
            Some(detail) => {
                gaps.push(ControllerNeed {
                    detail: detail.to_owned(),
                });
                "gap recorded".into()
            }
            None => {
                gaps.push(tool_decode_need(name, arguments, "no detail was given"));
                "arguments recorded as a gap".into()
            }
        },
        PatchToolShape::Declare { variant, fixed } => {
            if draft.declarations.len() >= MAX_DRAFT_DECLARATIONS {
                gaps.push(tool_decode_need(
                    name,
                    arguments,
                    "the draft is at its declaration cap",
                ));
                return "the draft is full".into();
            }
            let mut value = fields;
            value.insert("type".into(), Value::String(variant.into()));
            for (key, fixed_value) in fixed {
                value.insert((*key).into(), Value::String((*fixed_value).into()));
            }
            match serde_json::from_value::<Declaration>(Value::Object(value)) {
                Ok(declaration) => {
                    if let Declaration::Fact(fact) = &declaration
                        && let FactStandingRef::Canonical { evidence } = &fact.standing
                    {
                        draft.evidence.push(evidence.clone());
                    }
                    draft.declarations.push(declaration);
                    "declaration captured".into()
                }
                Err(error) => {
                    gaps.push(tool_decode_need(name, arguments, &error.to_string()));
                    "arguments recorded as a gap".into()
                }
            }
        }
        PatchToolShape::Operate { variant } => {
            if draft.operations.len() >= MAX_DRAFT_OPERATIONS {
                gaps.push(tool_decode_need(
                    name,
                    arguments,
                    "the draft is at its operation cap",
                ));
                return "the draft is full".into();
            }
            let mut value = fields;
            value.insert("op".into(), Value::String(variant.into()));
            match serde_json::from_value::<ComponentOp>(Value::Object(value)) {
                Ok(operation) => {
                    if let ComponentOp::Admit { evidence, .. } = &operation {
                        draft.evidence.push(evidence.clone());
                    }
                    draft.operations.push(operation);
                    "operation captured".into()
                }
                Err(error) => {
                    gaps.push(tool_decode_need(name, arguments, &error.to_string()));
                    "arguments recorded as a gap".into()
                }
            }
        }
    }
}

/// A citation outside the round's retrieved set never reaches the kernel as
/// provenance: the draft carries no such reference, so an `admit` that cited it
/// is refused on `AdmitWithoutEvidence` rather than minting quantity.
fn filter_evidence(mut draft: WorldPatch, allowed: &BTreeSet<EvidenceRef>) -> WorldPatch {
    draft
        .evidence
        .retain(|reference| allowed.contains(reference));
    draft
}

pub(super) fn derive_elaboration_capture(
    prompt: &str,
    last_mismatches: &[Mismatch],
    completed: &[InferenceOutput],
) -> Result<ElaborationCapture, ControllerError> {
    match evaluate_elaboration_loop(prompt, last_mismatches, completed)? {
        ElaborationLoopEvaluation::Complete { capture } => Ok(capture),
        ElaborationLoopEvaluation::Continue { .. } => Err(ControllerError::Serialization(
            "elaboration evidence did not finalize".into(),
        )),
    }
}

pub(super) fn elaboration_request(
    command_id: CommandId,
    round: usize,
    model: &str,
    input: Vec<CodexInputItem>,
) -> Result<InferenceRequest, ControllerError> {
    tool_request(
        command_id,
        round,
        InferencePurpose::Elaboration,
        model,
        ELABORATION_INSTRUCTIONS,
        input,
        patch_tools(),
    )
}

/// A session's identity is derived from the world, the jurisdiction, and the
/// answer's digest, so a crashed loop resumes the same store row and the
/// mailbox's idempotency probe sees the same command. No `uuid/v5`: the digest
/// is sha256 and its first sixteen bytes become the id.
fn session_command_id(session: &ElaboratorSession) -> Result<CommandId, ControllerError> {
    let mut hasher = Sha256::new();
    hasher.update(ELABORATION_NAMESPACE.as_bytes());
    hasher.update(
        rmp_serde::to_vec_named(&(
            session.world_id,
            session.jurisdiction,
            &session.answer_digest,
        ))
        .map_err(|error| ControllerError::Serialization(error.to_string()))?,
    );
    let bytes: [u8; 16] = hasher.finalize()[..16]
        .try_into()
        .expect("sha256 yields at least sixteen bytes");
    CommandId::parse_uuid(&Uuid::from_bytes(bytes).to_string())
        .map_err(|error| ControllerError::Serialization(error.to_string()))
}

fn digest_of<T: Serialize>(value: &T) -> Result<String, ControllerError> {
    let bytes = rmp_serde::to_vec_named(value)
        .map_err(|error| ControllerError::Serialization(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(calls: Vec<(&str, Value)>) -> InferenceOutput {
        InferenceOutput {
            events: calls
                .into_iter()
                .enumerate()
                .map(|(index, (name, arguments))| InferenceEvent::ToolCall {
                    call_id: format!("call-{index}"),
                    name: name.to_owned(),
                    arguments: arguments.to_string(),
                })
                .collect(),
            receipt_digest: "sha256:receipt".into(),
        }
    }

    fn place(handle: &str) -> Value {
        serde_json::json!({"handle": handle, "label": "A Shed", "container": null})
    }

    fn capture(completed: &[InferenceOutput]) -> ElaborationCapture {
        derive_elaboration_capture("prompt", &[], completed).expect("the loop finalizes")
    }

    /// Each declaration tool call appends a `Declaration`; each operation call
    /// appends a `ComponentOp`; `submit` is terminal.
    #[test]
    fn a_draft_accumulates_in_call_order_and_submit_is_terminal() {
        let completed = vec![output(vec![
            ("declare_place", place("north")),
            ("declare_place", place("south")),
            (
                "open_route",
                serde_json::json!({"route": {"ref": "draft", "value": "hatch"}}),
            ),
            (SUBMIT_PATCH_TOOL, serde_json::json!({})),
        ])];
        let capture = capture(&completed);
        assert!(capture.submitted);
        assert_eq!(capture.draft.declarations.len(), 2);
        assert_eq!(capture.draft.operations.len(), 1);
        assert!(capture.gaps.is_empty());
        // The model's sequencing is the patch's sequencing; nothing reorders.
        assert!(matches!(
            &capture.draft.declarations[0],
            Declaration::Entity(entity) if entity.label == "A Shed"
        ));
    }

    /// A malformed call, an unknown tool, and a second submit are gaps. None of
    /// them reaches the kernel.
    #[test]
    fn a_malformed_or_duplicate_call_becomes_a_gap() {
        let completed = vec![output(vec![
            ("declare_place", serde_json::json!({"handle": 7})),
            ("invent_a_god", serde_json::json!({})),
            ("declare_place", place("north")),
            (SUBMIT_PATCH_TOOL, serde_json::json!({})),
            (SUBMIT_PATCH_TOOL, serde_json::json!({})),
        ])];
        let capture = capture(&completed);
        assert!(capture.submitted);
        assert_eq!(capture.draft.declarations.len(), 1);
        assert_eq!(capture.gaps.len(), 3);
        assert!(capture.gaps[2].detail.contains("submitted twice"));
    }

    /// The author owns the size of what it authors. The kernel gets no cap.
    #[test]
    fn a_draft_past_the_size_cap_becomes_a_gap() {
        let mut calls: Vec<(&str, Value)> = (0..=MAX_DRAFT_DECLARATIONS)
            .map(|index| {
                (
                    "declare_place",
                    serde_json::json!({
                        "handle": format!("shed_{index}"),
                        "label": "A Shed",
                        "container": null,
                    }),
                )
            })
            .collect();
        calls.push((SUBMIT_PATCH_TOOL, serde_json::json!({})));
        let capture = capture(&vec![output(calls)]);
        assert_eq!(capture.draft.declarations.len(), MAX_DRAFT_DECLARATIONS);
        assert!(
            capture.gaps.iter().any(|gap| gap.detail.contains("cap")),
            "the call past the cap was not recorded as a gap"
        );
    }

    /// Provenance has an owner and it is the runner: a citation outside the
    /// round's retrieved set never reaches the kernel as evidence.
    #[test]
    fn a_cited_evidence_ref_outside_the_retrieved_set_is_dropped() {
        let completed = vec![output(vec![
            (
                "admit",
                serde_json::json!({
                    "holder": {"ref": "existing", "value": "00000000-0000-0000-0000-000000000001"},
                    "resource": {"ref": "existing", "value": "00000000-0000-0000-0000-000000000002"},
                    "qty": 3,
                    "evidence": "vault:forged",
                }),
            ),
            (SUBMIT_PATCH_TOOL, serde_json::json!({})),
        ])];
        let capture = capture(&completed);
        assert_eq!(
            capture.draft.evidence,
            vec![EvidenceRef::new("vault:forged")]
        );
        let allowed = BTreeSet::from([EvidenceRef::new("vault:harvest")]);
        let filtered = filter_evidence(capture.draft, &allowed);
        assert!(filtered.evidence.is_empty());
        // The operation survives and is refused by the resolver's own gate.
        assert_eq!(filtered.operations.len(), 1);
    }

    /// The round budget is a fixed point, not a retry: a model that never
    /// submits ends in `NoPatch` with its gaps, and the boundary stays derived.
    #[test]
    fn a_fruitless_session_ends_without_a_patch() {
        let completed: Vec<InferenceOutput> = (0..ELABORATION_ROUND_BUDGET)
            .map(|_| {
                output(vec![(
                    RECORD_GAP_PATCH_TOOL,
                    serde_json::json!({"detail": "no lore"}),
                )])
            })
            .collect();
        let capture = capture(&completed);
        assert!(!capture.submitted);
        assert!(capture.draft.declarations.is_empty());
        assert!(capture.gaps.last().unwrap().detail.contains("round budget"));
    }

    /// A boundary that survives a repair keeps its digest and therefore its
    /// command id, so a crashed loop resumes the same store row. A different
    /// answer is a different id.
    #[test]
    fn a_session_identity_is_derived_from_its_answer() {
        let session = |digest: &str| ElaboratorSession {
            world_id: WorldId::nil_for_test(),
            jurisdiction: JurisdictionKey::Uncovered,
            answer: PatchAnswer::Deficit(JurisdictionKey::Uncovered),
            answer_digest: BoundaryDigest::from_digest(digest.to_owned()),
            ancestry: "sha256:ancestry".into(),
        };
        let first = session_command_id(&session("sha256:one")).unwrap();
        assert_eq!(first, session_command_id(&session("sha256:one")).unwrap());
        assert_ne!(first, session_command_id(&session("sha256:two")).unwrap());
    }
}
