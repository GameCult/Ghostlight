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
    InferenceRequest, PreparedInference, RequestShape, canonical_model, prepared_matches_request,
    tool_decode_need, tool_request,
};
#[cfg(test)]
use super::patch::RECORD_GAP_PATCH_TOOL;
use super::patch::{self, ComponentOp, FactStandingRef};
use super::patch::{
    PATCH_TOOLS, PatchToolShape, SUBMIT_PATCH_TOOL, patch_tool_signatures, patch_tools,
};
use super::{
    BoundaryDigest, CausalBoundary, CommandId, Declaration, ElaborationPort, EntityId, EvidenceRef,
    JurisdictionKey, KernelError, MailboxError, Mismatch, PatchAnswer, ScaleDeficitRow, SeedPort,
    SubjectId, SubjectKind, WorldId, WorldPatch, WorldPhase, WorldSnapshot,
};
use async_trait::async_trait;
use codex_connector::CodexInputItem;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use thiserror::Error;
#[cfg(test)]
use uuid::Uuid;

/// The elaborator wants many tool calls inside one round and several rounds for
/// repair, where the operational agent wants one terminal choice.
/// `TOOL_STEP_BUDGET` is left alone for the lane it belongs to.
const ELABORATION_ROUND_BUDGET: usize = 6;
/// A seed session authors a whole shortfall, not one answer: several subjects,
/// each with a grant, a goal, and an obligation, then a submit. Measured on
/// the road at one declaration per response, six rounds discarded everything.
const SEED_ROUND_BUDGET: usize = 24;

const ELABORATION_NAMESPACE: &str = "ghostlight.command.elaboration.v1";

const ELABORATION_INSTRUCTIONS: &str = "Use only the supplied tools to author structure inside your jurisdiction. Answer the boundary or deficit you were given, then submit. Recording a gap changes nothing.";

/// The answer a session is bound to, plus the ancestry it was built against.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ElaboratorSession {
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
pub(crate) enum ElaborationCheckpoint {
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
                    && match evaluate_elaboration_loop(
                        agent_prompt,
                        last_mismatches,
                        completed,
                        ELABORATION_ROUND_BUDGET,
                    ) {
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
            } => derive_elaboration_capture(
                agent_prompt,
                last_mismatches,
                completed,
                ELABORATION_ROUND_BUDGET,
            )
            .is_ok_and(|capture| capture.submitted),
            Self::NoPatch {
                agent_prompt,
                completed,
                ..
            } => derive_elaboration_capture(agent_prompt, &[], completed, ELABORATION_ROUND_BUDGET)
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
pub(crate) struct EvidenceError {
    detail: String,
}

/// The referents the answer names, as world labels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EvidenceQuery {
    pub(crate) referents: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EvidenceReceipt {
    /// The exact reference an admitted patch may carry.
    pub(crate) reference: EvidenceRef,
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
pub(crate) trait EvidenceSource: Send + Sync {
    async fn retrieve(&self, query: &EvidenceQuery) -> Result<Vec<EvidenceReceipt>, EvidenceError>;
}

/// What `runtime.rs` supplies until a retrieval organ lands. The bound is real
/// and stated rather than hidden: with no receipts an elaborator can declare
/// structure and operate on it, but cannot mint quantity, because `admit`
/// requires a listed reference and there are none. Creation of quantity without
/// provenance should be impossible, not merely discouraged.
pub(crate) struct NullEvidenceSource;

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
            match evaluate_elaboration_loop(
                &agent_prompt,
                &last_mismatches,
                &completed,
                ELABORATION_ROUND_BUDGET,
            )? {
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
                    // `check_patch_caps` is the one enforcement site for the
                    // three item caps on a `WorldPatch` value; the consumer
                    // ingress reaches it through `decode_patch`, and this is
                    // the elaboration lane's own call into the same function,
                    // so `MAX_PATCH_EVIDENCE` (never checked per-tool-call
                    // above, unlike declarations and operations before this
                    // pass) is bound here too.
                    if let Err(error) = patch::check_patch_caps(&draft) {
                        return Err(ControllerError::Serialization(format!(
                            "elaboration draft exceeded a patch item cap: {error:?}"
                        )));
                    }
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
        let conversation = match evaluate_elaboration_loop(
            agent_prompt,
            last_mismatches,
            &[],
            ELABORATION_ROUND_BUDGET,
        )? {
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
    prompt_body(&mut prompt, receipts, last_mismatches);
    prompt
}

/// The half of an authoring prompt that is neither jurisdiction- nor
/// phase-specific: what may be cited, what must be repaired, and which tools
/// exist. One owner, so the two lanes cannot drift on what a citation rule says
/// or on which tools are offered.
fn prompt_body(prompt: &mut String, receipts: &[EvidenceReceipt], last_mismatches: &[Mismatch]) {
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
    budget: usize,
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

        let is_complete = submitted || !called_tool || round + 1 == budget;
        if is_complete {
            if round + 1 != completed.len() {
                return Err(ControllerError::Serialization(
                    "elaboration evidence continued after total finalization".into(),
                ));
            }
            if !submitted && called_tool && round + 1 == budget {
                // What was authored is still a patch; the resolver decides it,
                // not the round counter. An empty draft has nothing to submit.
                if draft.declarations.is_empty() && draft.operations.is_empty() {
                    gaps.push(ControllerNeed {
                        detail: "The elaboration round budget ended before a submit.".into(),
                    });
                } else {
                    submitted = true;
                    gaps.push(ControllerNeed {
                        detail: "The round budget ended before a submit; the draft as authored was submitted.".into(),
                    });
                }
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

    if completed.len() >= budget {
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
    budget: usize,
) -> Result<ElaborationCapture, ControllerError> {
    match evaluate_elaboration_loop(prompt, last_mismatches, completed, budget)? {
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
        RequestShape {
            // A patch turn emits many tool calls where a decision turn emits
            // one, so the budget belongs to the caller.
            max_output_tokens: 4_000,
            parallel_tool_calls: false,
        },
    )
}

/// A session's identity is derived from the world, the jurisdiction, and the
/// answer's digest, so a crashed loop resumes the same store row and the
/// mailbox's idempotency probe sees the same command. The derivation itself is
/// `CommandId::derived`, which is the one recipe every derived command key
/// uses.
///
/// Pass 10 moved this derivation onto `CommandId::derived`, which hashes a
/// different byte stream than the pass-9 spelling did (see
/// `soul_the_session_command_id_derivation_moved_under_the_refactor`). There is
/// no migration for command ids computed under the old spelling: a session
/// checkpointed before this pass would resume under an id the mailbox has
/// never seen, which is exactly the silent-resume failure a bumped schema is
/// for. The `controller_work` row and schema constants carry v9 for the same
/// reason the `consumer.v1` bump exists — a pre-refactor checkpoint refuses to
/// open rather than resuming under a mismatched id.
fn session_command_id(session: &ElaboratorSession) -> Result<CommandId, ControllerError> {
    let scope = digest_of(&(session.world_id, session.jurisdiction))?;
    Ok(CommandId::derived(
        ELABORATION_NAMESPACE,
        &[&scope, session.answer_digest.text()],
    ))
}

fn digest_of<T: Serialize>(value: &T) -> Result<String, ControllerError> {
    let bytes = rmp_serde::to_vec_named(value)
        .map_err(|error| ControllerError::Serialization(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

// ---- The seed lane -------------------------------------------------------
//
// Draft's authoring session, beside Active's. It shares this file's round loop,
// tool application, evidence filter, cap check, repair prompt, and checkpoint
// discipline; it differs in three places and no fourth — the phase it runs in,
// the row it selects, and the port it submits through.

const SEED_NAMESPACE: &str = "ghostlight.command.seed.v1";

/// A goal commitment carries no counterparty and can never derive a
/// `MissingStructure` boundary — a promise to oneself needs no forum. A world
/// seeded out of goals alone therefore activates with nothing for the Active
/// elaborator to answer, so this instruction requires an obligation, not a
/// goal alone, from every subject it authors.
const SEED_INSTRUCTIONS: &str = "Use only the supplied tools to author living structure inside your jurisdiction. A subject counts only when it has a controller, at least one affordance grant, and holds a goal commitment. A goal alone is not enough structure: every person and institution you author must also hold at least one obligation commitment with a counterparty, because a goal carries no counterparty and derives no boundary for the world's Active elaborator to answer. Author the shortfall you were given in as few responses as you can, making several tool calls per response, and finish with submit_patch as your last call; a session that ends without it keeps only what was authored. Recording a gap changes nothing.";

/// The row a session answers, plus the ancestry it was built against. There is
/// no `PatchAnswer` here: a seed answers nothing, and giving this session an
/// answer field it must never submit would be a field that lies.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SeedSession {
    pub(super) world_id: WorldId,
    pub(super) jurisdiction: JurisdictionKey,
    pub(super) kind: SubjectKind,
    pub(super) target: u32,
    pub(super) qualified: u32,
    /// The commit digest the draft is built against, so each landed patch opens
    /// a fresh session and the sweep advances instead of resubmitting one id.
    pub(super) ancestry: String,
}

/// Derived from the world, the row, and the ancestry, so a crashed loop resumes
/// the same store row without a registry. `patch::derive_id` keeps exactly two
/// call sites: this is a command key, not a referent id.
fn seed_command_id(session: &SeedSession) -> Result<CommandId, ControllerError> {
    let scope = digest_of(&(session.world_id, session.jurisdiction, session.kind))?;
    Ok(CommandId::derived(
        SEED_NAMESPACE,
        &[&scope, &session.ancestry],
    ))
}

/// `ElaborationCheckpoint`'s stages with `SeedSession` substituted and nothing
/// added. The draft is not a field, for the same reason it is not one there: it
/// is re-derived from `completed`, so a resumed session cannot submit a draft
/// the conversation does not produce.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "stage", rename_all = "snake_case")]
pub(crate) enum SeedCheckpoint {
    SeedInFlight {
        command_id: CommandId,
        session: SeedSession,
        agent_prompt: String,
        last_mismatches: Vec<Mismatch>,
        completed: Vec<InferenceOutput>,
        invocation: PreparedInference,
    },
    ReadyToSubmit {
        command_id: CommandId,
        session: SeedSession,
        agent_prompt: String,
        last_mismatches: Vec<Mismatch>,
        completed: Vec<InferenceOutput>,
    },
    /// The round budget ran out, or the model finished without a submit. The
    /// shortfall stays derived; nothing is repaired by waiting.
    NoPatch {
        command_id: CommandId,
        session: SeedSession,
        agent_prompt: String,
        completed: Vec<InferenceOutput>,
        gaps: Vec<ControllerNeed>,
    },
}

impl SeedCheckpoint {
    pub(super) fn command_id(&self) -> CommandId {
        match self {
            Self::SeedInFlight { command_id, .. }
            | Self::ReadyToSubmit { command_id, .. }
            | Self::NoPatch { command_id, .. } => *command_id,
        }
    }

    fn session(&self) -> &SeedSession {
        match self {
            Self::SeedInFlight { session, .. }
            | Self::ReadyToSubmit { session, .. }
            | Self::NoPatch { session, .. } => session,
        }
    }

    pub(super) fn is_initial(&self) -> bool {
        matches!(
            self,
            Self::SeedInFlight {
                completed,
                last_mismatches,
                ..
            } if completed.is_empty() && last_mismatches.is_empty()
        )
    }

    pub(super) fn integrity_is_valid(&self) -> bool {
        match self {
            Self::SeedInFlight {
                command_id,
                agent_prompt,
                last_mismatches,
                completed,
                invocation,
                ..
            } => {
                !agent_prompt.is_empty()
                    && canonical_model(&invocation.invocation.request.model)
                    && match evaluate_elaboration_loop(
                        agent_prompt,
                        last_mismatches,
                        completed,
                        SEED_ROUND_BUDGET,
                    ) {
                        Ok(ElaborationLoopEvaluation::Continue { conversation }) => seed_request(
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
                        }),
                        Ok(ElaborationLoopEvaluation::Complete { .. }) | Err(_) => false,
                    }
            }
            Self::ReadyToSubmit {
                agent_prompt,
                last_mismatches,
                completed,
                ..
            } => derive_elaboration_capture(
                agent_prompt,
                last_mismatches,
                completed,
                SEED_ROUND_BUDGET,
            )
            .is_ok_and(|capture| capture.submitted),
            Self::NoPatch {
                agent_prompt,
                completed,
                ..
            } => derive_elaboration_capture(agent_prompt, &[], completed, SEED_ROUND_BUDGET)
                .is_ok_and(|capture| !capture.submitted),
        }
    }
}

/// A session may gather more evidence and may end, but it may never rewrite the
/// row it bound to.
pub(super) fn valid_seed_progression(existing: &SeedCheckpoint, next: &SeedCheckpoint) -> bool {
    if existing.command_id() != next.command_id() || existing.session() != next.session() {
        return false;
    }
    match (existing, next) {
        (
            SeedCheckpoint::SeedInFlight {
                completed: existing,
                ..
            },
            SeedCheckpoint::SeedInFlight {
                completed: next, ..
            }
            | SeedCheckpoint::ReadyToSubmit {
                completed: next, ..
            }
            | SeedCheckpoint::NoPatch {
                completed: next, ..
            },
        ) => next.len() >= existing.len() && next.starts_with(existing),
        // A rejection reopens the same command id for repair. A rejected
        // command mutates nothing, so the round evidence starts over.
        (
            SeedCheckpoint::ReadyToSubmit { .. },
            SeedCheckpoint::SeedInFlight {
                last_mismatches, ..
            },
        ) => !last_mismatches.is_empty(),
        _ => false,
    }
}

/// What one seed `step` did, for the transport that asked for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SeedOutcome {
    /// Every row's deficit is zero: the terminating condition.
    Clean,
    /// The world is not Draft. Seeding is Draft's lane exactly as elaboration
    /// is Active's.
    NotDraft,
    Committed,
    /// The patch committed and the row's deficit did not strictly fall. A fixed
    /// point, not a retry.
    NoProgress,
    /// The kernel returned a complete mismatch set; the next round repairs it.
    Rejected,
    /// The world moved under the session. Re-select; this is not a retry loop.
    Superseded,
    /// The round budget was spent without a submit.
    NoPatch,
}

impl SeedOutcome {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::NotDraft => "not_draft",
            Self::Committed => "committed",
            Self::NoProgress => "no_progress",
            Self::Rejected => "rejected",
            Self::Superseded => "superseded",
            Self::NoPatch => "no_patch",
        }
    }
}

/// The Draft authoring loop. Holds no state between steps; every field is a
/// port, an identity, or the owner's own one-sentence brief.
pub(crate) struct SeedRunner {
    mailbox: SeedPort,
    inference: Arc<dyn InferencePort>,
    evidence: Arc<dyn EvidenceSource>,
    work: Arc<dyn ControllerWorkStore>,
    model: String,
    brief: Option<String>,
}

impl SeedRunner {
    pub(super) fn new(
        mailbox: SeedPort,
        inference: Arc<dyn InferencePort>,
        evidence: Arc<dyn EvidenceSource>,
        work: Arc<dyn ControllerWorkStore>,
        model: String,
        brief: Option<String>,
    ) -> Self {
        Self {
            mailbox,
            inference,
            evidence,
            work,
            model,
            brief,
        }
    }

    /// One shortfall row, start to finish.
    pub(crate) async fn step(&self) -> Result<SeedOutcome, ControllerError> {
        let snapshot = self.mailbox.snapshot().await.map_err(snapshot_error)?;
        if snapshot.phase != WorldPhase::Draft {
            return Ok(SeedOutcome::NotDraft);
        }
        let Some(row) = select_row(&snapshot) else {
            return Ok(SeedOutcome::Clean);
        };
        let session = SeedSession {
            world_id: snapshot.world_id,
            jurisdiction: row.jurisdiction,
            kind: row.kind,
            target: row.target,
            qualified: row.qualified,
            ancestry: snapshot.last_commit_digest.clone().unwrap_or_default(),
        };
        let command_id = seed_command_id(&session)?;

        let receipts = self
            .evidence
            .retrieve(&EvidenceQuery {
                referents: seed_referents(&snapshot, &session, self.brief.as_deref()),
            })
            .await
            .map_err(|error| ControllerError::WorkPersistence(error.to_string()))?;
        let allowed: BTreeSet<EvidenceRef> = receipts
            .iter()
            .map(|receipt| receipt.reference.clone())
            .collect();

        let existing = match self.work.lookup(command_id).await? {
            ControllerWorkLookup::Missing => None,
            ControllerWorkLookup::Confirmed(ControllerWork::Seed(checkpoint))
            | ControllerWorkLookup::CustodyUncertain(ControllerWork::Seed(checkpoint)) => {
                if checkpoint.session() != &session {
                    return Ok(SeedOutcome::Superseded);
                }
                Some(checkpoint)
            }
            ControllerWorkLookup::Confirmed(_) | ControllerWorkLookup::CustodyUncertain(_) => {
                return Err(ControllerError::CommandMismatch);
            }
        };

        let (agent_prompt, mut last_mismatches, mut completed) = match existing {
            Some(SeedCheckpoint::NoPatch { .. }) => return Ok(SeedOutcome::NoPatch),
            Some(
                SeedCheckpoint::SeedInFlight {
                    agent_prompt,
                    last_mismatches,
                    completed,
                    ..
                }
                | SeedCheckpoint::ReadyToSubmit {
                    agent_prompt,
                    last_mismatches,
                    completed,
                    ..
                },
            ) => (agent_prompt, last_mismatches, completed),
            None => (
                build_seed_prompt(&snapshot, &session, self.brief.as_deref(), &receipts, &[]),
                Vec::new(),
                Vec::new(),
            ),
        };

        loop {
            match evaluate_elaboration_loop(
                &agent_prompt,
                &last_mismatches,
                &completed,
                SEED_ROUND_BUDGET,
            )? {
                ElaborationLoopEvaluation::Continue { conversation } => {
                    let request =
                        seed_request(command_id, completed.len(), &self.model, conversation)?;
                    let invocation = self.inference.prepare(request).map_err(|source| {
                        ControllerError::Inference {
                            purpose: InferencePurpose::Elaboration,
                            source,
                        }
                    })?;
                    self.persist(SeedCheckpoint::SeedInFlight {
                        command_id,
                        session: session.clone(),
                        agent_prompt: agent_prompt.clone(),
                        last_mismatches: last_mismatches.clone(),
                        completed: completed.clone(),
                        invocation: invocation.clone(),
                    })
                    .await?;
                    let output = self.inference.infer(invocation).await.map_err(|source| {
                        ControllerError::Inference {
                            purpose: InferencePurpose::Elaboration,
                            source,
                        }
                    })?;
                    completed.push(output);
                }
                ElaborationLoopEvaluation::Complete { capture } => {
                    if !capture.submitted {
                        let last = completed
                            .last()
                            .map(|output| format!("{output:?}"))
                            .unwrap_or_default();
                        tracing::info!(
                            rounds = completed.len(),
                            gaps = ?capture.gaps,
                            last_output = %last.chars().take(2000).collect::<String>(),
                            "seed session ended without a patch"
                        );
                        self.persist(SeedCheckpoint::NoPatch {
                            command_id,
                            session,
                            agent_prompt,
                            completed,
                            gaps: capture.gaps,
                        })
                        .await?;
                        return Ok(SeedOutcome::NoPatch);
                    }
                    self.persist(SeedCheckpoint::ReadyToSubmit {
                        command_id,
                        session: session.clone(),
                        agent_prompt: agent_prompt.clone(),
                        last_mismatches: last_mismatches.clone(),
                        completed: completed.clone(),
                    })
                    .await?;
                    let draft = filter_evidence(capture.draft, &allowed);
                    if let Err(error) = patch::check_patch_caps(&draft) {
                        return Err(ControllerError::Serialization(format!(
                            "seed draft exceeded a patch item cap: {error:?}"
                        )));
                    }
                    let submitted = self
                        .mailbox
                        .submit_seed(command_id, snapshot.world_id, draft)
                        .await;
                    return match submitted {
                        // Termination is the deficit, not a counter the runner
                        // keeps: a commit that did not strictly lower this
                        // row's shortfall is a fixed point.
                        Ok(_) => {
                            let after = self.mailbox.snapshot().await.map_err(snapshot_error)?;
                            let remaining = row_deficit(&after, &session);
                            Ok(if remaining < row.deficit {
                                SeedOutcome::Committed
                            } else {
                                SeedOutcome::NoProgress
                            })
                        }
                        Err(MailboxError::Kernel(KernelError::PatchRejected(mismatches))) => {
                            tracing::info!(
                                mismatches = ?mismatches,
                                "seed patch rejected; a repair prompt is persisted for the next step"
                            );
                            last_mismatches = mismatches;
                            let repaired = build_seed_prompt(
                                &snapshot,
                                &session,
                                self.brief.as_deref(),
                                &receipts,
                                &last_mismatches,
                            );
                            self.persist_repair(command_id, &session, &repaired, &last_mismatches)
                                .await?;
                            Ok(SeedOutcome::Rejected)
                        }
                        Err(MailboxError::Kernel(KernelError::RevisionMismatch { .. })) => {
                            Ok(SeedOutcome::Superseded)
                        }
                        // The world left Draft while this round's inference was
                        // in flight; `submit_seed` caught it against its own
                        // fresh snapshot rather than the one this step began
                        // with.
                        Err(MailboxError::Kernel(KernelError::WrongPhase { .. })) => {
                            Ok(SeedOutcome::NotDraft)
                        }
                        Err(error) => Err(snapshot_error(error)),
                    };
                }
            }
        }
    }

    /// One bounded sweep. The budget is the caller's, because who may spend a
    /// paid endpoint is a transport question and how far a deficit has fallen
    /// is not.
    pub(crate) async fn sweep(&self, sessions: usize) -> Result<SeedOutcome, ControllerError> {
        let mut last = SeedOutcome::Clean;
        for _ in 0..sessions {
            last = self.step().await?;
            if !matches!(last, SeedOutcome::Committed) {
                break;
            }
        }
        Ok(last)
    }

    async fn persist(&self, checkpoint: SeedCheckpoint) -> Result<(), ControllerError> {
        match self.work.persist(&ControllerWork::Seed(checkpoint)).await? {
            ControllerWorkWrite::Applied | ControllerWorkWrite::AlreadyPresent => Ok(()),
            ControllerWorkWrite::CustodyUncertain => Err(ControllerError::WorkPersistence(
                "controller work custody is uncertain".into(),
            )),
        }
    }

    async fn persist_repair(
        &self,
        command_id: CommandId,
        session: &SeedSession,
        agent_prompt: &str,
        last_mismatches: &[Mismatch],
    ) -> Result<(), ControllerError> {
        let conversation =
            match evaluate_elaboration_loop(agent_prompt, last_mismatches, &[], SEED_ROUND_BUDGET)?
            {
                ElaborationLoopEvaluation::Continue { conversation } => conversation,
                ElaborationLoopEvaluation::Complete { .. } => {
                    return Err(ControllerError::Serialization(
                        "a repair round completed before any evidence".into(),
                    ));
                }
            };
        let request = seed_request(command_id, 0, &self.model, conversation)?;
        let invocation =
            self.inference
                .prepare(request)
                .map_err(|source| ControllerError::Inference {
                    purpose: InferencePurpose::Elaboration,
                    source,
                })?;
        self.persist(SeedCheckpoint::SeedInFlight {
            command_id,
            session: session.clone(),
            agent_prompt: agent_prompt.to_owned(),
            last_mismatches: last_mismatches.to_vec(),
            completed: Vec::new(),
            invocation,
        })
        .await
    }
}

/// The first shortfall the world reports, in the order the snapshot reports
/// them. The panel calls this too, so the card and the runner cannot disagree
/// about what happens next.
pub(crate) fn select_row(snapshot: &WorldSnapshot) -> Option<ScaleDeficitRow> {
    snapshot
        .scale_deficit
        .iter()
        .find(|row| row.deficit > 0)
        .copied()
}

fn row_deficit(snapshot: &WorldSnapshot, session: &SeedSession) -> u32 {
    snapshot
        .scale_deficit
        .iter()
        .find(|row| row.jurisdiction == session.jurisdiction && row.kind == session.kind)
        .map_or(0, |row| row.deficit)
}

fn seed_request(
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
        SEED_INSTRUCTIONS,
        input,
        patch_tools(),
        RequestShape {
            // A seed response carries many declarations and operations at
            // once; one call per response spent the whole budget on the road.
            max_output_tokens: 8_000,
            parallel_tool_calls: true,
        },
    )
}

/// What the Vault is asked about. Deliberately not `answer_referents`, which
/// returns nothing for the uncovered residual — the only jurisdiction a genesis
/// world has. A one-room world retrieves against its own title and the labels
/// of the places that already exist rather than against nothing.
fn seed_referents(
    snapshot: &WorldSnapshot,
    session: &SeedSession,
    brief: Option<&str>,
) -> Vec<String> {
    let mut referents = Vec::new();
    match session.jurisdiction {
        JurisdictionKey::PlaceSubtree(root) => referents.extend(
            snapshot
                .places
                .iter()
                .find(|entry| entry.id == root)
                .map(|entry| entry.label.clone()),
        ),
        JurisdictionKey::Uncovered => referents.push(snapshot.title.clone()),
    }
    if let Some(brief) = brief.map(str::trim).filter(|value| !value.is_empty()) {
        referents.push(brief.to_owned());
    }
    referents.extend(
        snapshot
            .places
            .iter()
            .take(8)
            .map(|entry| entry.label.clone()),
    );
    referents.dedup();
    referents
}

fn build_seed_prompt(
    snapshot: &WorldSnapshot,
    session: &SeedSession,
    brief: Option<&str>,
    receipts: &[EvidenceReceipt],
    last_mismatches: &[Mismatch],
) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are seeding a world before it opens. Nothing here is running yet.\n\n");
    prompt.push_str(&format!("World: {}\n", snapshot.title));
    prompt.push_str(&format!(
        "Jurisdiction: {}\n",
        render_jurisdiction(snapshot, session.jurisdiction)
    ));
    prompt.push_str(&format!(
        "Shortfall: this jurisdiction is meant to hold {} {} who are alive in it; {} qualify today. Author the rest.\n",
        session.target,
        render_kind(session.kind),
        session.qualified
    ));
    // The row is a place subtree; a subject standing anywhere else fills a
    // row with no target and leaves this shortfall untouched. On the road the
    // first landed seed put all six people in the commons for want of this.
    prompt.push_str(&match session.jurisdiction {
        JurisdictionKey::PlaceSubtree(root) => format!(
            "Placement: every subject you author must stand at {} [{}] or at a place you declare inside it; a subject standing anywhere else does not count toward this shortfall.\n",
            render_jurisdiction(snapshot, session.jurisdiction),
            id_text(root)
        ),
        JurisdictionKey::Uncovered => {
            "Placement: every subject you author must stand at a place no declared root covers.\n".to_owned()
        }
    });
    if let Some(brief) = brief.map(str::trim).filter(|value| !value.is_empty()) {
        prompt.push_str(&format!("{brief}\n"));
    }
    prompt.push_str(
        "\nA subject qualifies only when it has a controller, at least one affordance grant, and holds a goal commitment. Declare people, institutions, and populations who want something; give each a controller, grants, a position, and a goal. Give them the rest of a life: routines and obligations that recur, counterparties, channels they speak on and who controls them, authority and the offices that lend it, holdings and the dependencies those holdings serve, routes between the places they move through. A subject with no counterparty who can command or litigate its goal leaves a boundary for the elaborator; that is allowed and expected, not an error.\nYou may not declare a human-controlled subject: only the world's first person is human, and that was genesis.\n\n",
    );
    prompt.push_str(&render_world_structure(snapshot));
    prompt.push('\n');
    prompt_body(&mut prompt, receipts, last_mismatches);
    prompt
}

fn render_kind(kind: SubjectKind) -> &'static str {
    match kind {
        SubjectKind::Person => "persons",
        SubjectKind::Institution => "institutions",
        SubjectKind::Population => "populations",
    }
}

/// The world the session is authoring into, rendered from snapshot fields that
/// already exist. A renderer beside `render_jurisdiction` and `render_answer`,
/// and seed-only: widening the Active elaborator's prompt is a behaviour change
/// to a working lane and belongs to whoever measures it.
/// The canonical id as the patch vocabulary spells it: the bare UUID text,
/// without the typed wrapper's name.
fn id_text(id: impl std::fmt::Debug) -> String {
    let text = format!("{id:?}");
    match (text.find('('), text.rfind(')')) {
        (Some(open), Some(close)) if open < close => text[open + 1..close].to_owned(),
        _ => text,
    }
}

fn render_world_structure(snapshot: &WorldSnapshot) -> String {
    let place_label = |id: EntityId| {
        snapshot
            .places
            .iter()
            .find(|entry| entry.id == id)
            .map_or_else(
                || "an unnamed place".to_owned(),
                |entry| entry.label.clone(),
            )
    };
    // Ids are printed beside labels because a patch names an existing thing
    // by its canonical id and nothing else; a model that sees only labels
    // can only guess, and the first live seed session guessed six times.
    let mut out = String::from(
        "Standing structure (reference an existing thing by the id in brackets, \
         exactly as printed; reference a thing declared in this patch by its handle):\n",
    );
    out.push_str("  Places:");
    if snapshot.places.is_empty() {
        out.push_str(" none");
    }
    for place in &snapshot.places {
        match place.container {
            Some(container) => {
                out.push_str(&format!(
                    " {} [{}] (in {});",
                    place.label,
                    id_text(place.id),
                    place_label(container)
                ));
            }
            None => out.push_str(&format!(" {} [{}];", place.label, id_text(place.id))),
        }
    }
    out.push_str("\n  Routes:");
    if snapshot.routes.is_empty() {
        out.push_str(" none");
    }
    for route in &snapshot.routes {
        out.push_str(&format!(
            " {} [{}]: {} -> {}, {:?}, {};",
            route.label,
            id_text(route.id),
            place_label(route.from),
            place_label(route.to),
            route.access,
            if route.open { "open" } else { "closed" }
        ));
    }
    out.push_str("\n  Subjects:");
    if snapshot.subjects.is_empty() {
        out.push_str(" none");
    }
    for subject in &snapshot.subjects {
        out.push_str(&format!(
            " {} [{}] ({:?}, {}, in {}, grants: {}, {});",
            subject.label,
            id_text(subject.id),
            subject.kind,
            subject
                .controller_mode
                .map_or_else(|| "external".to_owned(), |mode| format!("{mode:?}")),
            subject
                .position
                .map_or_else(|| "nowhere".to_owned(), place_label),
            subject.affordances.len(),
            if subject.qualified {
                "counts"
            } else {
                "does not count"
            }
        ));
    }
    out.push_str("\n  Affordances:");
    if snapshot.affordances.is_empty() {
        out.push_str(" none");
    }
    for affordance in &snapshot.affordances {
        out.push_str(&format!(
            " {} [{}], roles: {}, {};",
            affordance.entry.kind.0,
            id_text(affordance.id),
            if affordance.entry.roles.is_empty() {
                "none".to_owned()
            } else {
                affordance
                    .entry
                    .roles
                    .iter()
                    .map(|role| role.role.0.clone())
                    .collect::<Vec<_>>()
                    .join("/")
            },
            if affordance.entry.carries_speech {
                "speech"
            } else {
                "silent"
            }
        ));
    }
    out.push_str("\n  Resources:");
    if snapshot.resources.is_empty() {
        out.push_str(" none");
    }
    for resource in &snapshot.resources {
        out.push_str(&format!(" {};", resource.label));
    }
    out.push_str("\n  Shortfall rows:");
    if snapshot.scale_deficit.is_empty() {
        out.push_str(" none");
    }
    for row in &snapshot.scale_deficit {
        out.push_str(&format!(
            " {} {}: target {}, qualified {}, short {};",
            render_jurisdiction(snapshot, row.jurisdiction),
            render_kind(row.kind),
            row.target,
            row.qualified,
            row.deficit
        ));
    }
    out.push('\n');
    out
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
        derive_elaboration_capture("prompt", &[], completed, ELABORATION_ROUND_BUDGET)
            .expect("the loop finalizes")
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

    /// Soul, pass 10. `session_command_id` was rewritten onto
    /// `CommandId::derived`, and the two spellings hash different byte streams:
    /// the old one hashed one msgpack tuple of `(world, jurisdiction, digest)`,
    /// the new one hashes a length-prefixed pair of `(digest_of(world,
    /// jurisdiction), digest text)`. The derivation is still deterministic and
    /// still keyed on the same three inputs, but a session checkpointed before
    /// this pass resumes under a different command id. That is a silent id
    /// migration for in-flight elaboration rows, and this test pins the fact so
    /// it cannot move again unnoticed.
    #[test]
    fn soul_the_session_command_id_derivation_moved_under_the_refactor() {
        let session = ElaboratorSession {
            world_id: WorldId::nil_for_test(),
            jurisdiction: JurisdictionKey::Uncovered,
            answer: PatchAnswer::Deficit(JurisdictionKey::Uncovered),
            answer_digest: BoundaryDigest::from_digest("sha256:deadbeef".into()),
            ancestry: "sha256:ancestry".into(),
        };
        let live = session_command_id(&session).unwrap();
        assert_eq!(live, session_command_id(&session).unwrap());

        // The pass-9 spelling, reproduced exactly.
        let mut hasher = Sha256::new();
        hasher.update(ELABORATION_NAMESPACE.as_bytes());
        hasher.update(
            rmp_serde::to_vec_named(&(
                session.world_id,
                session.jurisdiction,
                &session.answer_digest,
            ))
            .unwrap(),
        );
        let bytes: [u8; 16] = hasher.finalize()[..16].try_into().unwrap();
        let previous = CommandId::parse_uuid(&Uuid::from_bytes(bytes).to_string()).unwrap();
        assert_ne!(
            live, previous,
            "the derivation is unchanged; delete this test and the migration note"
        );

        // And the new recipe still separates the inputs it is keyed on.
        let moved = ElaboratorSession {
            answer_digest: BoundaryDigest::from_digest("sha256:cafe".into()),
            ..session.clone()
        };
        assert_ne!(live, session_command_id(&moved).unwrap());
    }

    /// The author owns the size of what it authors. The kernel gets no cap.
    #[test]
    fn a_draft_past_the_declaration_cap_is_refused_by_the_shared_cap_check() {
        // The elaboration lane no longer caps a declaration mid-round: one
        // enforcement site, `check_patch_caps`, catches an over-cap draft
        // before it reaches submission (see `step`), so at this layer the
        // draft grows past the cap uncapped.
        let mut calls: Vec<(&str, Value)> = (0..=patch::MAX_PATCH_DECLARATIONS)
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
        assert_eq!(
            capture.draft.declarations.len(),
            patch::MAX_PATCH_DECLARATIONS + 1
        );
        assert_eq!(
            patch::check_patch_caps(&capture.draft),
            Err(patch::PatchDecodeError::TooManyDeclarations {
                count: patch::MAX_PATCH_DECLARATIONS + 1
            })
        );
    }

    /// `MAX_PATCH_EVIDENCE` has no pre-append check at all: every admitted
    /// citation is pushed to the draft unconditionally as it is captured.
    /// `check_patch_caps` is the one place that bites, and it bites at the same
    /// function every other cap goes through -- there is no separate,
    /// unenforced evidence path on the elaboration lane.
    #[test]
    fn an_over_evidence_draft_is_refused_by_the_shared_cap_check() {
        let mut calls: Vec<(&str, Value)> = (0..=patch::MAX_PATCH_EVIDENCE)
            .map(|index| {
                (
                    "admit",
                    serde_json::json!({
                        "holder": {"ref": "existing", "value": "00000000-0000-0000-0000-000000000001"},
                        "resource": {"ref": "existing", "value": "00000000-0000-0000-0000-000000000002"},
                        "qty": 1,
                        "evidence": format!("vault:harvest-{index}"),
                    }),
                )
            })
            .collect();
        calls.push((SUBMIT_PATCH_TOOL, serde_json::json!({})));
        let capture = capture(&vec![output(calls)]);
        assert_eq!(capture.draft.evidence.len(), patch::MAX_PATCH_EVIDENCE + 1);
        assert_eq!(
            patch::check_patch_caps(&capture.draft),
            Err(patch::PatchDecodeError::TooManyEvidence {
                count: patch::MAX_PATCH_EVIDENCE + 1
            })
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

    // ---- Soul --------------------------------------------------------

    /// The identity's preimage is (world, jurisdiction, answer digest) and all
    /// three separate. The ancestry is deliberately not in it for the boundary
    /// lane: a boundary that survives a repair keeps its id, which is what
    /// makes the resume idempotent rather than orphaning a store row.
    #[test]
    fn soul_a_session_identity_separates_world_jurisdiction_and_answer() {
        let root = EntityId(Uuid::from_u128(1));
        let sibling = EntityId(Uuid::from_u128(2));
        let session =
            |world: WorldId, jurisdiction: JurisdictionKey, digest: &str, ancestry: &str| {
                ElaboratorSession {
                    world_id: world,
                    jurisdiction,
                    answer: PatchAnswer::Deficit(jurisdiction),
                    answer_digest: BoundaryDigest::from_digest(digest.to_owned()),
                    ancestry: ancestry.to_owned(),
                }
            };
        let base = session(
            WorldId::nil_for_test(),
            JurisdictionKey::PlaceSubtree(root),
            "sha256:one",
            "sha256:ancestry",
        );
        let id = |value: &ElaboratorSession| session_command_id(value).unwrap();

        assert_ne!(
            id(&base),
            id(&session(
                WorldId::nil_for_test(),
                JurisdictionKey::PlaceSubtree(sibling),
                "sha256:one",
                "sha256:ancestry",
            )),
            "two jurisdictions on one world share an identity"
        );
        assert_eq!(
            id(&base),
            id(&session(
                WorldId::nil_for_test(),
                JurisdictionKey::PlaceSubtree(root),
                "sha256:one",
                "sha256:later",
            )),
            "the boundary lane's identity moved when only the ancestry did"
        );
    }

    /// The pass ungates `EvidenceRef::new`, so the question is whether a
    /// model-authored patch can now mint a canonical fact on a receipt nobody
    /// retrieved. It cannot: the citation is carried into `patch.evidence` by
    /// the evaluator and then dropped by `filter_evidence`, and the resolver
    /// refuses a canonical fact whose reference the patch does not list.
    #[test]
    fn soul_a_canonical_fact_cannot_be_minted_on_an_unretrieved_receipt() {
        let completed = vec![output(vec![
            (
                "declare_fact",
                serde_json::json!({
                    "handle": "flood",
                    "label": "The Flooded Hinge",
                    "statement": "The lower hinge flooded.",
                    "standing": {"standing": "canonical", "evidence": "vault:forged"},
                }),
            ),
            (SUBMIT_PATCH_TOOL, serde_json::json!({})),
        ])];
        let capture = capture(&completed);
        assert_eq!(
            capture.draft.evidence,
            vec![EvidenceRef::new("vault:forged")],
            "the citation is carried, so the runner is the one that must drop it"
        );

        // Nothing was retrieved this round, which is what `NullEvidenceSource`
        // supplies in production today.
        let filtered = filter_evidence(capture.draft, &BTreeSet::new());
        assert!(filtered.evidence.is_empty());
        // The declaration survives with its reference intact, which is exactly
        // the shape `Mismatch::FactWithoutEvidence` refuses.
        assert_eq!(filtered.declarations.len(), 1);
        assert!(matches!(
            &filtered.declarations[0],
            Declaration::Fact(fact)
                if matches!(&fact.standing, FactStandingRef::Canonical { evidence }
                    if evidence == &EvidenceRef::new("vault:forged"))
        ));
    }
}
