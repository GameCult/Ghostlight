use super::{
    AgencyGraph, AuthenticatedCaller, CallerId, CommandBody, CommandEnvelope, CommandId,
    ConsumerId, CreateWorld, CreateWorldIntent, CreationReceipt, DecisionInvocation,
    DecisionOpportunity, Declaration, DraftHandle, EntityDeclaration, EntityKind, JurisdictionKey,
    KernelError, NewController, OperatorEvent, PatchAnswer, PrincipalCommandIntent, PrincipalId,
    Ref, SubjectDeclaration, SubjectKind, SubmitReceipt, SystemCapability, TickMinutes, WorldId,
    WorldKernel, WorldPatch, WorldScaleIntentRef, WorldSnapshot, journal,
    patch::kernel_speak_grant, prepare_creation,
};
use crate::app_session::VerifiedPrincipalEvidence;
use std::path::Path;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

/// The one place a world starts with, so its first subjects can hear each
/// other. A world that wants more declares them through the patch lane.
const GENESIS_PLACE: &str = "commons";

const REQUEST_CAPACITY: usize = 32;

#[derive(Clone, Debug)]
pub(crate) struct WorldMailbox {
    sender: mpsc::Sender<Request>,
}

enum OwnedWorld {
    Empty(journal::EmptyWorldJournal),
    Live(WorldKernel),
}

enum Request {
    Create {
        input: CreateWorld,
        authenticated: AuthenticatedCaller,
        reply: oneshot::Sender<Result<CreationReceipt, KernelError>>,
    },
    Submit {
        command: CommandEnvelope,
        authenticated: AuthenticatedCaller,
        reply: oneshot::Sender<Result<SubmitReceipt, KernelError>>,
    },
    /// An opportunity-bearing command. Its scope digest is the whole binding, so
    /// the envelope revision is stamped here, inside the owner task, where there
    /// is no race — a caller-supplied revision would be a second, stricter
    /// binding for a proposal the digest already binds.
    SubmitStamped {
        command_id: CommandId,
        caller: CallerId,
        body: CommandBody,
        authenticated: AuthenticatedCaller,
        reply: oneshot::Sender<Result<SubmitReceipt, KernelError>>,
    },
    Snapshot {
        reply: oneshot::Sender<Result<WorldSnapshot, KernelError>>,
    },
    /// The human operator's story feed. Separate from `Snapshot` so no
    /// controller lane can reach an unscoped event log.
    OperatorLog {
        reply: oneshot::Sender<Result<Vec<OperatorEvent>, KernelError>>,
    },
    /// The scheduler's adjacency projection. Separate from `Snapshot` for the
    /// same reason the story feed is: it is wider than any subject view, and
    /// only the tick driver may hold it.
    AgencyGraph {
        reply: oneshot::Sender<Result<AgencyGraph, KernelError>>,
    },
    ControllerReceipt {
        command_id: CommandId,
        opportunity: super::DecisionOpportunity,
        invocation: super::DecisionInvocation,
        reply: oneshot::Sender<Result<Option<super::CommitReceipt>, KernelError>>,
    },
    ControllerDeclineReceipt {
        command_id: CommandId,
        opportunity: super::DecisionOpportunity,
        reply: oneshot::Sender<Result<Option<super::CommitReceipt>, KernelError>>,
    },
}

#[derive(Debug, Error)]
pub(crate) enum MailboxError {
    #[error("world owner mailbox is unavailable")]
    Unavailable,
    #[error("world command outcome is unknown after enqueue: {command_id:?}")]
    OutcomeUnknown { command_id: CommandId },
    #[error(transparent)]
    Kernel(#[from] KernelError),
}

impl WorldMailbox {
    pub(crate) fn open(path: impl AsRef<Path>) -> Result<(Self, JoinHandle<()>), KernelError> {
        let owned = match journal::WorldJournal::open_owner(path.as_ref())? {
            journal::JournalOpen::Empty(empty) => OwnedWorld::Empty(empty),
            journal::JournalOpen::Live { journal, state } => {
                OwnedWorld::Live(WorldKernel { state, journal })
            }
        };
        let (sender, receiver) = mpsc::channel(REQUEST_CAPACITY);
        let task = tokio::spawn(run_owner(owned, receiver));
        Ok((Self { sender }, task))
    }

    pub(crate) async fn create(
        &self,
        input: CreateWorldIntent,
        principal: &VerifiedPrincipalEvidence,
    ) -> Result<CreationReceipt, MailboxError> {
        let principal_id = PrincipalId::new(principal.account_subject_hash());
        // Ingress owns label normalization; the reducer only admits canonical
        // labels.
        // A world needs a room. Co-located speech fills the place its speaker
        // stands in, so ingress declares one and stands every genesis subject
        // there rather than creating a world where nobody can be heard.
        let declare = |handle: &str, label: String, kind, controller| {
            Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new(handle),
                label: label.trim().to_owned(),
                kind,
                controller,
                affordances: kernel_speak_grant(),
                position: Some(Ref::Draft(DraftHandle::new(GENESIS_PLACE))),
            })
        };
        // Roots stand beside the commons, not inside it: a root contained by
        // the commons would make every commons subject count toward that root
        // through `covers_place`, and the world's first person belongs to no
        // jurisdiction's population target. The owner subject lands in
        // `Uncovered`, visible and reducing nothing.
        let mut declarations = vec![
            Declaration::Entity(EntityDeclaration {
                handle: DraftHandle::new(GENESIS_PLACE),
                label: "The Commons".into(),
                kind: EntityKind::Place,
                container: None,
            }),
            declare(
                "first-person",
                input.human_subject_label,
                SubjectKind::Person,
                NewController::Human {
                    principal: principal_id.clone(),
                },
            ),
        ];
        if let Some(label) = input.narrative_persona_label {
            declarations.push(declare(
                "narrative-persona",
                label,
                SubjectKind::Person,
                NewController::NarrativePersona,
            ));
        }
        if let Some(label) = input.operational_agent_label {
            declarations.push(declare(
                "operational-agent",
                label,
                SubjectKind::Institution,
                NewController::OperationalAgent,
            ));
        }
        for root in &input.jurisdictions {
            declarations.push(Declaration::Entity(EntityDeclaration {
                handle: DraftHandle::new(root.handle.clone()),
                label: root.label.trim().to_owned(),
                kind: EntityKind::Place,
                container: None,
            }));
        }
        let scale_intent = WorldScaleIntentRef {
            targets: input.targets,
            jurisdictions: input
                .jurisdictions
                .iter()
                .map(|root| (DraftHandle::new(root.handle.clone()), root.permille))
                .collect(),
        };
        self.create_authenticated(
            CreateWorld {
                id: input.id,
                owner: principal_id.clone(),
                title: input.title,
                patch: WorldPatch {
                    declarations,
                    operations: Vec::new(),
                    evidence: Vec::new(),
                },
                scale_intent,
            },
            AuthenticatedCaller::verified_principal(principal_id),
        )
        .await
    }

    async fn create_authenticated(
        &self,
        input: CreateWorld,
        authenticated: AuthenticatedCaller,
    ) -> Result<CreationReceipt, MailboxError> {
        let command_id = input.id;
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Request::Create {
                input,
                authenticated,
                reply,
            })
            .await
            .map_err(|_| MailboxError::Unavailable)?;

        response
            .await
            .map_err(|_| MailboxError::OutcomeUnknown { command_id })?
            .map_err(mailbox_outcome)
    }

    pub(crate) async fn submit_principal(
        &self,
        intent: PrincipalCommandIntent,
        principal: &VerifiedPrincipalEvidence,
    ) -> Result<SubmitReceipt, MailboxError> {
        let principal_id = PrincipalId::new(principal.account_subject_hash());
        let caller = CallerId::Principal(principal_id.clone());
        let authenticated = AuthenticatedCaller::verified_principal(principal_id);
        if matches!(
            intent.body,
            CommandBody::ExerciseDecision { .. } | CommandBody::DeclineDecision { .. }
        ) {
            return self
                .submit_stamped(intent.id, caller, intent.body, authenticated)
                .await;
        }
        self.submit_authenticated(
            CommandEnvelope {
                id: intent.id,
                world_id: intent.world_id,
                expected_revision: intent.expected_revision,
                caller,
                body: intent.body,
            },
            authenticated,
        )
        .await
    }

    /// Stamps both `world_id` and `expected_revision` from the live kernel. The
    /// mailbox owns exactly one world, so there is no other world a caller could
    /// have meant, and asking a controller runner to restate an ID it cannot
    /// choose would only invent a way to be wrong. The consequence is that
    /// `WorldKernel::submit`'s `WorldMismatch` check cannot fire on this path:
    /// scope, not world identity, is what fails an opportunity-bearing command
    /// here, and `soul_a_stamped_submission_still_fails_closed_on_a_foreign_scope`
    /// is the proof that it still fails closed.
    async fn submit_stamped(
        &self,
        command_id: CommandId,
        caller: CallerId,
        body: CommandBody,
        authenticated: AuthenticatedCaller,
    ) -> Result<SubmitReceipt, MailboxError> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Request::SubmitStamped {
                command_id,
                caller,
                body,
                authenticated,
                reply,
            })
            .await
            .map_err(|_| MailboxError::Unavailable)?;

        response
            .await
            .map_err(|_| MailboxError::OutcomeUnknown { command_id })?
            .map_err(mailbox_outcome)
    }

    async fn submit_authenticated(
        &self,
        command: CommandEnvelope,
        authenticated: AuthenticatedCaller,
    ) -> Result<SubmitReceipt, MailboxError> {
        let command_id = command.id;
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Request::Submit {
                command,
                authenticated,
                reply,
            })
            .await
            .map_err(|_| MailboxError::Unavailable)?;

        response
            .await
            .map_err(|_| MailboxError::OutcomeUnknown { command_id })?
            .map_err(mailbox_outcome)
    }

    /// Controller-only commit port. This method is visible inside the world
    /// subtree, where the controller organ lives, but not to runtime ingress.
    pub(super) async fn submit_controller(
        &self,
        command_id: CommandId,
        opportunity: &DecisionOpportunity,
        invocation: DecisionInvocation,
    ) -> Result<SubmitReceipt, MailboxError> {
        let controller_id = opportunity.controller_id;
        self.submit_stamped(
            command_id,
            CallerId::Controller(controller_id),
            CommandBody::ExerciseDecision {
                opportunity: opportunity.clone(),
                invocation,
            },
            AuthenticatedCaller::verified_controller(controller_id),
        )
        .await
    }

    pub(super) async fn submit_controller_decline(
        &self,
        command_id: CommandId,
        opportunity: &DecisionOpportunity,
    ) -> Result<SubmitReceipt, MailboxError> {
        let controller_id = opportunity.controller_id;
        self.submit_stamped(
            command_id,
            CallerId::Controller(controller_id),
            CommandBody::DeclineDecision {
                opportunity: opportunity.clone(),
            },
            AuthenticatedCaller::verified_controller(controller_id),
        )
        .await
    }

    /// The clock port. Stamped, like every opportunity-bearing command, because
    /// the tick task cannot know the live revision either. It is visible inside
    /// the world subtree and unreachable from runtime ingress, which builds
    /// `CallerId::Principal` from verified evidence and nothing else.
    pub(crate) async fn submit_clock(
        &self,
        command_id: CommandId,
        minutes: TickMinutes,
    ) -> Result<SubmitReceipt, MailboxError> {
        self.submit_stamped(
            command_id,
            CallerId::System(SystemCapability::Clock),
            CommandBody::AdvanceTime { minutes },
            AuthenticatedCaller::verified_system(SystemCapability::Clock),
        )
        .await
    }

    /// The elaborator's only commit port. Stamped, like every digest-binding
    /// command: the loop cannot know the live revision either, and an
    /// `AdmitPatch` from an elaborator binds by its answer's digest exactly as
    /// an `ExerciseDecision` binds by scope digest. `answers` is not an
    /// `Option`, so the port cannot express an unanswered elaborator patch and
    /// `AnswerRequired` is reachable only through the owner lane and the
    /// journal. Visible inside the world subtree, where the elaboration organ
    /// lives; unreachable from runtime ingress.
    pub(super) async fn submit_elaboration(
        &self,
        command_id: CommandId,
        jurisdiction: JurisdictionKey,
        answers: PatchAnswer,
        patch: WorldPatch,
    ) -> Result<SubmitReceipt, MailboxError> {
        self.submit_stamped(
            command_id,
            CallerId::System(SystemCapability::Elaborator { jurisdiction }),
            CommandBody::AdmitPatch {
                answers: Some(answers),
                patch,
            },
            AuthenticatedCaller::verified_system(SystemCapability::Elaborator { jurisdiction }),
        )
        .await
    }

    /// The consumer lane's one door, and the only constructor of a `Consumer`
    /// capability. It takes the revision the consumer built against rather than
    /// stamping one: a caller that cannot name its revision cannot be told its
    /// batch is stale, and telling it so is the whole point of the field.
    pub(super) async fn submit_consumer(
        &self,
        world_id: WorldId,
        expected_revision: u64,
        command_id: CommandId,
        consumer: ConsumerId,
        answers: Option<PatchAnswer>,
        patch: WorldPatch,
    ) -> Result<SubmitReceipt, MailboxError> {
        let capability = SystemCapability::Consumer { consumer };
        self.submit_authenticated(
            CommandEnvelope {
                id: command_id,
                world_id,
                expected_revision,
                caller: CallerId::System(capability),
                body: CommandBody::AdmitPatch { answers, patch },
            },
            AuthenticatedCaller::verified_system(capability),
        )
        .await
    }

    #[cfg(test)]
    pub(super) async fn create_fixture(
        &self,
        input: CreateWorld,
        authenticated: &AuthenticatedCaller,
    ) -> Result<CreationReceipt, MailboxError> {
        self.create_authenticated(input, authenticated.clone())
            .await
    }

    #[cfg(test)]
    pub(super) async fn submit_fixture(
        &self,
        command: CommandEnvelope,
        authenticated: &AuthenticatedCaller,
    ) -> Result<SubmitReceipt, MailboxError> {
        self.submit_authenticated(command, authenticated.clone())
            .await
    }

    /// The human operator's story feed. It is not a perception surface, and no
    /// controller lane calls it.
    /// The tick driver's adjacency projection. It is on `WorldMailbox` and on
    /// neither `ControllerPort` nor `ElaborationPort`: a controller organ that
    /// tried to read adjacency would fail to compile.
    pub(crate) async fn agency_graph(&self) -> Result<AgencyGraph, MailboxError> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Request::AgencyGraph { reply })
            .await
            .map_err(|_| MailboxError::Unavailable)?;

        response
            .await
            .map_err(|_| MailboxError::Unavailable)?
            .map_err(MailboxError::Kernel)
    }

    pub(crate) async fn operator_log(&self) -> Result<Vec<OperatorEvent>, MailboxError> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Request::OperatorLog { reply })
            .await
            .map_err(|_| MailboxError::Unavailable)?;

        response
            .await
            .map_err(|_| MailboxError::Unavailable)?
            .map_err(MailboxError::Kernel)
    }

    pub(crate) async fn snapshot(&self) -> Result<WorldSnapshot, MailboxError> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Request::Snapshot { reply })
            .await
            .map_err(|_| MailboxError::Unavailable)?;

        response
            .await
            .map_err(|_| MailboxError::Unavailable)?
            .map_err(MailboxError::Kernel)
    }

    pub(super) async fn controller_receipt(
        &self,
        command_id: CommandId,
        opportunity: &super::DecisionOpportunity,
        invocation: &super::DecisionInvocation,
    ) -> Result<Option<super::CommitReceipt>, MailboxError> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Request::ControllerReceipt {
                command_id,
                opportunity: opportunity.clone(),
                invocation: invocation.clone(),
                reply,
            })
            .await
            .map_err(|_| MailboxError::Unavailable)?;

        response
            .await
            .map_err(|_| MailboxError::Unavailable)?
            .map_err(MailboxError::Kernel)
    }

    pub(super) async fn controller_decline_receipt(
        &self,
        command_id: CommandId,
        opportunity: &super::DecisionOpportunity,
    ) -> Result<Option<super::CommitReceipt>, MailboxError> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Request::ControllerDeclineReceipt {
                command_id,
                opportunity: opportunity.clone(),
                reply,
            })
            .await
            .map_err(|_| MailboxError::Unavailable)?;

        response
            .await
            .map_err(|_| MailboxError::Unavailable)?
            .map_err(MailboxError::Kernel)
    }
}

/// The narrow port `ControllerRunner` is built against, instead of the whole
/// `WorldMailbox`. It forwards exactly the five requests a controller lane
/// makes today — a snapshot to read from, and the four commit paths a
/// controller may exercise or decline through — and names nothing else. In
/// particular it has no `operator_log` method: "no controller lane calls the
/// human operator's story feed" used to be a convention about who calls a
/// `pub(crate)` method on `WorldMailbox`. Now it is a fact about which methods
/// this type has. Adding `.operator_log()` to `controllers.rs` fails to
/// compile, because `ControllerPort` never named it; that failure is the
/// proof, not a test that runs and passes.
/// The elaboration organ's narrowing of the mailbox: it reads a snapshot and
/// commits one authored patch. Nothing else on `WorldMailbox` is reachable from
/// the authoring lane, and that is a fact about which methods this type has
/// rather than a test that runs and passes.
#[derive(Clone)]
pub(crate) struct ElaborationPort {
    mailbox: WorldMailbox,
}

impl ElaborationPort {
    pub(crate) fn new(mailbox: WorldMailbox) -> Self {
        Self { mailbox }
    }

    pub(super) async fn snapshot(&self) -> Result<WorldSnapshot, MailboxError> {
        self.mailbox.snapshot().await
    }

    pub(super) async fn submit_elaboration(
        &self,
        command_id: CommandId,
        jurisdiction: JurisdictionKey,
        answers: PatchAnswer,
        patch: WorldPatch,
    ) -> Result<SubmitReceipt, MailboxError> {
        self.mailbox
            .submit_elaboration(command_id, jurisdiction, answers, patch)
            .await
    }
}

/// The seeding organ's narrowing of the mailbox. It reads a snapshot and
/// commits one unanswered Draft patch as the owner. It has no `operator_log`,
/// no `agency_graph`, no `submit_clock`, and no `ApproveDraft`/`ActivateWorld`:
/// those are not methods on this type, so reaching for one fails to compile.
///
/// It carries the owner's `VerifiedPrincipalEvidence` for the session's
/// lifetime because a multi-round session cannot re-derive it: the only minter
/// is `AppSessionOwner` holding a live cookie, and a checkpoint that stored an
/// account hash so the runner could re-mint one would be a second minter and an
/// offline forge path. The evidence is captured from the request that asked for
/// the work and dies with the port.
#[derive(Clone)]
pub(crate) struct SeedPort {
    mailbox: WorldMailbox,
    principal: VerifiedPrincipalEvidence,
}

impl SeedPort {
    pub(crate) fn new(mailbox: WorldMailbox, principal: VerifiedPrincipalEvidence) -> Self {
        Self { mailbox, principal }
    }

    pub(super) async fn snapshot(&self) -> Result<WorldSnapshot, MailboxError> {
        self.mailbox.snapshot().await
    }

    /// One body, hardcoded. The seed lane cannot express an answered patch, so
    /// it meets exactly one answer gate — `require_answer`'s Draft branch — and
    /// the owner arm of `require_patch_author`, which is unconfined.
    ///
    /// It takes `expected_revision` rather than stamping one: a world that
    /// moved under a long session fails `StaleRevision` and the runner reports
    /// `Superseded`, where a stamped submission would silently commit against a
    /// world the model never saw.
    pub(super) async fn submit_seed(
        &self,
        command_id: CommandId,
        world_id: WorldId,
        expected_revision: u64,
        patch: WorldPatch,
    ) -> Result<SubmitReceipt, MailboxError> {
        self.mailbox
            .submit_principal(
                PrincipalCommandIntent {
                    id: command_id,
                    world_id,
                    expected_revision,
                    body: CommandBody::AdmitPatch {
                        answers: None,
                        patch,
                    },
                },
                &self.principal,
            )
            .await
    }
}

/// The consumer ingress's narrowing of the mailbox: one method. The ingress
/// has no reason to read the world — it selects no answer, pre-validates
/// nothing, and returns a receipt rather than state — so a snapshot method here
/// would be authority granted before the pass that needs it.
#[derive(Clone)]
pub(crate) struct ConsumerPort {
    mailbox: WorldMailbox,
}

impl ConsumerPort {
    pub(crate) fn new(mailbox: WorldMailbox) -> Self {
        Self { mailbox }
    }

    pub(super) async fn submit_consumer(
        &self,
        world_id: WorldId,
        expected_revision: u64,
        command_id: CommandId,
        consumer: ConsumerId,
        answers: Option<PatchAnswer>,
        patch: WorldPatch,
    ) -> Result<SubmitReceipt, MailboxError> {
        self.mailbox
            .submit_consumer(
                world_id,
                expected_revision,
                command_id,
                consumer,
                answers,
                patch,
            )
            .await
    }
}

#[derive(Clone)]
pub(crate) struct ControllerPort {
    mailbox: WorldMailbox,
}

impl ControllerPort {
    pub(crate) fn new(mailbox: WorldMailbox) -> Self {
        Self { mailbox }
    }

    pub(crate) async fn snapshot(&self) -> Result<WorldSnapshot, MailboxError> {
        self.mailbox.snapshot().await
    }

    pub(crate) async fn controller_receipt(
        &self,
        command_id: CommandId,
        opportunity: &super::DecisionOpportunity,
        invocation: &super::DecisionInvocation,
    ) -> Result<Option<super::CommitReceipt>, MailboxError> {
        self.mailbox
            .controller_receipt(command_id, opportunity, invocation)
            .await
    }

    pub(crate) async fn controller_decline_receipt(
        &self,
        command_id: CommandId,
        opportunity: &super::DecisionOpportunity,
    ) -> Result<Option<super::CommitReceipt>, MailboxError> {
        self.mailbox
            .controller_decline_receipt(command_id, opportunity)
            .await
    }

    pub(crate) async fn submit_controller(
        &self,
        command_id: CommandId,
        opportunity: &DecisionOpportunity,
        invocation: DecisionInvocation,
    ) -> Result<SubmitReceipt, MailboxError> {
        self.mailbox
            .submit_controller(command_id, opportunity, invocation)
            .await
    }

    pub(crate) async fn submit_controller_decline(
        &self,
        command_id: CommandId,
        opportunity: &DecisionOpportunity,
    ) -> Result<SubmitReceipt, MailboxError> {
        self.mailbox
            .submit_controller_decline(command_id, opportunity)
            .await
    }
}

fn mailbox_outcome(error: KernelError) -> MailboxError {
    match error {
        KernelError::RecoveryRequired { command_id } => MailboxError::OutcomeUnknown { command_id },
        other => MailboxError::Kernel(other),
    }
}

async fn run_owner(mut owned: OwnedWorld, mut receiver: mpsc::Receiver<Request>) {
    while let Some(request) = receiver.recv().await {
        match request {
            Request::Create {
                input,
                authenticated,
                reply,
            } => {
                let prepared = match prepare_creation(input, &authenticated) {
                    Ok(prepared) => prepared,
                    Err(error) => {
                        let _ = reply.send(Err(error));
                        continue;
                    }
                };
                if let OwnedWorld::Live(kernel) = &owned {
                    let result = kernel.retry_creation(&prepared);
                    let owner_must_stop = result
                        .as_ref()
                        .is_err_and(KernelError::requires_owner_restart);
                    let _ = reply.send(result);
                    if owner_must_stop {
                        return;
                    }
                    continue;
                }
                let OwnedWorld::Empty(empty) = owned else {
                    unreachable!("live owner returned above")
                };
                match WorldKernel::initialize(empty, prepared) {
                    Ok((kernel, receipt)) => {
                        owned = OwnedWorld::Live(kernel);
                        let _ = reply.send(Ok(receipt));
                    }
                    Err(error) => {
                        // Initialization consumed the sole journal handle. Its
                        // durability is uncertain, so this owner must release
                        // the path and make callers reopen explicitly.
                        let _ = reply.send(Err(error));
                        return;
                    }
                }
            }
            Request::Submit {
                command,
                authenticated,
                reply,
            } => {
                let result = match &mut owned {
                    OwnedWorld::Empty(_) => Err(KernelError::WorldNotCreated),
                    OwnedWorld::Live(kernel) => kernel.submit(command, &authenticated),
                };
                let owner_must_stop = result
                    .as_ref()
                    .is_err_and(KernelError::requires_owner_restart);
                let _ = reply.send(result);
                if owner_must_stop {
                    return;
                }
            }
            Request::SubmitStamped {
                command_id,
                caller,
                body,
                authenticated,
                reply,
            } => {
                let result = match &mut owned {
                    OwnedWorld::Empty(_) => Err(KernelError::WorldNotCreated),
                    OwnedWorld::Live(kernel) => {
                        let command = CommandEnvelope {
                            id: command_id,
                            world_id: kernel.state.world_id,
                            expected_revision: kernel.state.revision,
                            caller,
                            body,
                        };
                        kernel.submit(command, &authenticated)
                    }
                };
                let owner_must_stop = result
                    .as_ref()
                    .is_err_and(KernelError::requires_owner_restart);
                let _ = reply.send(result);
                if owner_must_stop {
                    return;
                }
            }
            Request::Snapshot { reply } => {
                let result = match &owned {
                    OwnedWorld::Empty(_) => Err(KernelError::WorldNotCreated),
                    OwnedWorld::Live(kernel) => kernel.snapshot(),
                };
                let owner_must_stop = result
                    .as_ref()
                    .is_err_and(KernelError::requires_owner_restart);
                let _ = reply.send(result);
                if owner_must_stop {
                    return;
                }
            }
            Request::OperatorLog { reply } => {
                let result = match &owned {
                    OwnedWorld::Empty(_) => Err(KernelError::WorldNotCreated),
                    OwnedWorld::Live(kernel) => kernel.operator_log(),
                };
                let owner_must_stop = result
                    .as_ref()
                    .is_err_and(KernelError::requires_owner_restart);
                let _ = reply.send(result);
                if owner_must_stop {
                    return;
                }
            }
            Request::AgencyGraph { reply } => {
                let result = match &owned {
                    OwnedWorld::Empty(_) => Err(KernelError::WorldNotCreated),
                    OwnedWorld::Live(kernel) => kernel.agency_graph(),
                };
                let owner_must_stop = result
                    .as_ref()
                    .is_err_and(KernelError::requires_owner_restart);
                let _ = reply.send(result);
                if owner_must_stop {
                    return;
                }
            }
            Request::ControllerReceipt {
                command_id,
                opportunity,
                invocation,
                reply,
            } => {
                let result = match &owned {
                    OwnedWorld::Empty(_) => Ok(None),
                    OwnedWorld::Live(kernel) => {
                        kernel.controller_receipt(command_id, &opportunity, &invocation)
                    }
                };
                let owner_must_stop = result
                    .as_ref()
                    .is_err_and(KernelError::requires_owner_restart);
                let _ = reply.send(result);
                if owner_must_stop {
                    return;
                }
            }
            Request::ControllerDeclineReceipt {
                command_id,
                opportunity,
                reply,
            } => {
                let result = match &owned {
                    OwnedWorld::Empty(_) => Ok(None),
                    OwnedWorld::Live(kernel) => {
                        kernel.controller_decline_receipt(command_id, &opportunity)
                    }
                };
                let owner_must_stop = result
                    .as_ref()
                    .is_err_and(KernelError::requires_owner_restart);
                let _ = reply.send(result);
                if owner_must_stop {
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{
        CallerId, CommandBody, Declaration, DraftHandle, EntityDeclaration, EntityKind, Mismatch,
        NewController, PrincipalId, SubjectDeclaration, SubjectKind, WorldId,
    };
    use std::{collections::BTreeSet, path::PathBuf};
    use tempfile::TempDir;

    fn authenticated_owner() -> AuthenticatedCaller {
        AuthenticatedCaller::fixture(CallerId::Principal(PrincipalId::new("owner")))
    }

    fn creation(id: CommandId, title: &str) -> CreateWorld {
        CreateWorld {
            id,
            owner: PrincipalId::new("owner"),
            title: title.into(),
            patch: WorldPatch {
                declarations: vec![
                    Declaration::Entity(EntityDeclaration {
                        handle: DraftHandle::new(GENESIS_PLACE),
                        label: "The Commons".into(),
                        kind: EntityKind::Place,
                        container: None,
                    }),
                    Declaration::Subject(SubjectDeclaration {
                        handle: DraftHandle::new("operator"),
                        label: "Operator".into(),
                        kind: SubjectKind::Person,
                        controller: NewController::Human {
                            principal: PrincipalId::new("owner"),
                        },
                        affordances: kernel_speak_grant(),
                        position: Some(Ref::Draft(DraftHandle::new(GENESIS_PLACE))),
                    }),
                ],
                operations: Vec::new(),
                evidence: Vec::new(),
            },
            scale_intent: WorldScaleIntentRef::default(),
        }
    }

    async fn fixture() -> (
        TempDir,
        PathBuf,
        WorldMailbox,
        JoinHandle<()>,
        AuthenticatedCaller,
        CreateWorld,
        CreationReceipt,
    ) {
        let directory = tempfile::tempdir().expect("temporary world directory");
        let path = directory.path().join("world.cc");
        let authenticated = authenticated_owner();
        let input = creation(CommandId::new(), "Mailbox World");
        let (mailbox, task) = WorldMailbox::open(&path).expect("open empty world owner");
        let receipt = mailbox
            .create_fixture(input.clone(), &authenticated)
            .await
            .expect("create fixture world");
        (
            directory,
            path,
            mailbox,
            task,
            authenticated,
            input,
            receipt,
        )
    }

    fn approval_command(world_id: WorldId, revision: u64) -> CommandEnvelope {
        CommandEnvelope {
            id: CommandId::new(),
            world_id,
            expected_revision: revision,
            caller: CallerId::Principal(PrincipalId::new("owner")),
            body: CommandBody::ApproveDraft,
        }
    }

    #[tokio::test]
    async fn empty_owner_rejects_world_operations_then_accepts_creation() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let authenticated = authenticated_owner();
        let (mailbox, task) = WorldMailbox::open(&path).unwrap();
        assert!(matches!(
            mailbox.snapshot().await,
            Err(MailboxError::Kernel(KernelError::WorldNotCreated))
        ));
        assert!(matches!(
            mailbox
                .submit_fixture(approval_command(WorldId::issue(), 0), &authenticated)
                .await,
            Err(MailboxError::Kernel(KernelError::WorldNotCreated))
        ));

        let receipt = mailbox
            .create_fixture(creation(CommandId::new(), "Created"), &authenticated)
            .await
            .unwrap();
        assert_eq!(mailbox.snapshot().await.unwrap().world_id, receipt.world_id);

        drop(mailbox);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn invalid_creation_leaves_the_owner_empty() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let authenticated = authenticated_owner();
        let (mailbox, task) = WorldMailbox::open(&path).unwrap();
        let mut invalid = creation(CommandId::new(), "Invalid");
        invalid
            .patch
            .declarations
            .push(invalid.patch.declarations[1].clone());
        let Err(MailboxError::Kernel(KernelError::PatchRejected(rejected))) =
            mailbox.create_fixture(invalid, &authenticated).await
        else {
            panic!("expected a rejected creation patch");
        };
        assert_eq!(
            rejected,
            vec![Mismatch::DuplicateHandle {
                handle: DraftHandle::new("operator")
            }]
        );
        assert!(matches!(
            mailbox.snapshot().await,
            Err(MailboxError::Kernel(KernelError::WorldNotCreated))
        ));
        assert!(
            mailbox
                .create_fixture(creation(CommandId::new(), "Valid"), &authenticated)
                .await
                .is_ok()
        );

        drop(mailbox);
        task.await.unwrap();
    }

    /// A creation patch that declares only entities passes structural and
    /// reference resolution, so it must not reach the journal at all: an
    /// empty subject set at genesis is a rejected patch, not a corrupt
    /// journal that kills the owner actor.
    #[tokio::test]
    async fn genesis_without_a_subject_is_a_mismatch_not_a_journal_fault() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let authenticated = authenticated_owner();
        let (mailbox, task) = WorldMailbox::open(&path).unwrap();

        let mut subjectless = creation(CommandId::new(), "No Subject");
        subjectless.patch.declarations = vec![Declaration::Entity(EntityDeclaration {
            handle: DraftHandle::new("empty-hall"),
            label: "The Empty Hall".into(),
            kind: EntityKind::Place,
            container: None,
        })];

        let Err(MailboxError::Kernel(KernelError::PatchRejected(rejected))) =
            mailbox.create_fixture(subjectless, &authenticated).await
        else {
            panic!("expected a rejected creation patch");
        };
        assert_eq!(rejected, vec![Mismatch::NoDecisionSubject]);
        assert!(matches!(
            mailbox.snapshot().await,
            Err(MailboxError::Kernel(KernelError::WorldNotCreated))
        ));

        // The mailbox actor is still alive: a valid creation on the same
        // kernel still succeeds.
        assert!(
            mailbox
                .create_fixture(creation(CommandId::new(), "Valid"), &authenticated)
                .await
                .is_ok()
        );

        drop(mailbox);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn empty_restart_remains_empty_without_a_sidecar_identity() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("world.cc");
        let (mailbox, task) = WorldMailbox::open(&path).unwrap();
        assert!(matches!(
            mailbox.snapshot().await,
            Err(MailboxError::Kernel(KernelError::WorldNotCreated))
        ));
        drop(mailbox);
        task.await.unwrap();

        let (reopened, reopened_task) = WorldMailbox::open(&path).unwrap();
        assert!(matches!(
            reopened.snapshot().await,
            Err(MailboxError::Kernel(KernelError::WorldNotCreated))
        ));
        drop(reopened);
        reopened_task.await.unwrap();
    }

    #[tokio::test]
    async fn exact_creation_retry_survives_live_state_and_restart() {
        let (_directory, path, mailbox, task, authenticated, input, receipt) = fixture().await;
        assert_eq!(
            mailbox
                .create_fixture(input.clone(), &authenticated)
                .await
                .unwrap(),
            receipt
        );
        let before_restart = mailbox.snapshot().await.unwrap();
        drop(mailbox);
        task.await.unwrap();

        let (reopened, reopened_task) = WorldMailbox::open(&path).unwrap();
        assert_eq!(reopened.snapshot().await.unwrap(), before_restart);
        assert_eq!(
            reopened
                .create_fixture(input, &authenticated)
                .await
                .unwrap(),
            receipt
        );

        drop(reopened);
        reopened_task.await.unwrap();
    }

    #[tokio::test]
    async fn creation_id_collision_and_second_creation_fail_closed() {
        let (_directory, _path, mailbox, task, authenticated, input, _) = fixture().await;
        let mut collision = input.clone();
        collision.title = "Different payload".into();
        assert!(matches!(
            mailbox.create_fixture(collision, &authenticated).await,
            Err(MailboxError::Kernel(KernelError::CreationConflict))
        ));
        assert!(matches!(
            mailbox
                .create_fixture(creation(CommandId::new(), "Second world"), &authenticated)
                .await,
            Err(MailboxError::Kernel(KernelError::CreationTargetOccupied))
        ));

        drop(mailbox);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn submit_then_snapshot_share_one_serial_owner() {
        let (_directory, _path, mailbox, task, authenticated, _input, receipt) = fixture().await;

        let result = mailbox
            .submit_fixture(approval_command(receipt.world_id, 0), &authenticated)
            .await
            .expect("submit through mailbox");
        assert!(matches!(result, SubmitReceipt::Applied(_)));
        let snapshot = mailbox.snapshot().await.expect("snapshot through mailbox");
        assert_eq!(snapshot.revision, 1);
        assert_eq!(
            snapshot.draft_approvals,
            BTreeSet::from([PrincipalId::new("owner")])
        );

        drop(mailbox);
        task.await.expect("owner task exits after last sender");
    }

    #[tokio::test]
    async fn concurrent_same_revision_submissions_cannot_both_commit() {
        let (_directory, _path, mailbox, task, authenticated, _input, receipt) = fixture().await;
        let left = approval_command(receipt.world_id, 0);
        let right = approval_command(receipt.world_id, 0);

        let (left_result, right_result) = tokio::join!(
            mailbox.submit_fixture(left, &authenticated),
            mailbox.submit_fixture(right, &authenticated)
        );
        let applied = [&left_result, &right_result]
            .into_iter()
            .filter(|result| matches!(result, Ok(SubmitReceipt::Applied(_))))
            .count();
        let stale = [&left_result, &right_result]
            .into_iter()
            .filter(|result| {
                matches!(
                    result,
                    Err(MailboxError::Kernel(KernelError::RevisionMismatch { .. }))
                )
            })
            .count();
        assert_eq!((applied, stale), (1, 1));

        drop(mailbox);
        task.await.expect("owner task exits after last sender");
    }

    #[tokio::test]
    async fn canceled_submit_reply_does_not_cancel_enqueued_command() {
        let (_directory, path, mailbox, task, authenticated, _input, receipt) = fixture().await;
        let (reply, response) = oneshot::channel();
        mailbox
            .sender
            .send(Request::Submit {
                command: approval_command(receipt.world_id, 0),
                authenticated,
                reply,
            })
            .await
            .expect("enqueue command");
        drop(response);
        drop(mailbox);

        task.await.expect("owner drains queue before exit");
        let (reopened, reopened_task) = WorldMailbox::open(&path).expect("reopen released world");
        let snapshot = reopened.snapshot().await.expect("read durable state");
        assert_eq!(snapshot.revision, 1);
        assert_eq!(
            snapshot.draft_approvals,
            BTreeSet::from([PrincipalId::new("owner")])
        );
        drop(reopened);
        reopened_task.await.unwrap();
    }

    #[tokio::test]
    async fn submit_reply_loss_is_outcome_unknown() {
        let (sender, mut receiver) = mpsc::channel(1);
        let mailbox = WorldMailbox { sender };
        let owner = tokio::spawn(async move {
            if let Some(Request::Submit { reply, .. }) = receiver.recv().await {
                drop(reply);
            }
        });
        let result = mailbox
            .submit_fixture(
                approval_command(WorldId::issue(), 0),
                &authenticated_owner(),
            )
            .await;
        assert!(matches!(result, Err(MailboxError::OutcomeUnknown { .. })));
        owner.await.expect("test owner exits");
    }

    #[tokio::test]
    async fn create_reply_loss_is_outcome_unknown() {
        let (sender, mut receiver) = mpsc::channel(1);
        let mailbox = WorldMailbox { sender };
        let owner = tokio::spawn(async move {
            if let Some(Request::Create { reply, .. }) = receiver.recv().await {
                drop(reply);
            }
        });
        let result = mailbox
            .create_fixture(
                creation(CommandId::new(), "Uncertain"),
                &authenticated_owner(),
            )
            .await;
        assert!(matches!(result, Err(MailboxError::OutcomeUnknown { .. })));
        owner.await.expect("test owner exits");
    }

    #[tokio::test]
    async fn snapshot_reply_loss_is_unavailable() {
        let (sender, mut receiver) = mpsc::channel(1);
        let mailbox = WorldMailbox { sender };
        let owner = tokio::spawn(async move {
            if let Some(Request::Snapshot { reply }) = receiver.recv().await {
                drop(reply);
            }
        });
        let result = mailbox.snapshot().await;
        assert!(matches!(result, Err(MailboxError::Unavailable)));
        owner.await.expect("test owner exits");
    }

    #[tokio::test]
    async fn closed_mailbox_is_unavailable_before_enqueue() {
        let (sender, receiver) = mpsc::channel(1);
        drop(receiver);
        let mailbox = WorldMailbox { sender };
        let result = mailbox
            .submit_fixture(
                approval_command(WorldId::issue(), 0),
                &authenticated_owner(),
            )
            .await;
        assert!(matches!(result, Err(MailboxError::Unavailable)));
    }

    /// Soul falsification: the envelope revision is stamped inside the owner
    /// task, so a controller turn bound at revision N still commits after an
    /// unrelated commit moved the world to N+1. This exercises the mailbox
    /// path, not the kernel gate a hand-built envelope would reach.
    #[tokio::test]
    async fn soul_a_bound_opportunity_commits_through_the_mailbox_after_an_unrelated_commit() {
        let directory = tempfile::tempdir().unwrap();
        let (mailbox, _task) = WorldMailbox::open(directory.path().join("world.cc")).unwrap();
        let owner = PrincipalId::new("owner");
        let authenticated = authenticated_owner();
        // Two rooms, not one: a telling reaches its own room, so the bound
        // subject must stand somewhere the unrelated speaker does not.
        let declare = |handle: &str, label: &str, controller: NewController| {
            Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new(handle),
                label: label.into(),
                kind: SubjectKind::Person,
                controller,
                affordances: kernel_speak_grant(),
                position: Some(Ref::Draft(DraftHandle::new(format!("room-{handle}")))),
            })
        };
        let room = |handle: &str| {
            Declaration::Entity(EntityDeclaration {
                handle: DraftHandle::new(format!("room-{handle}")),
                label: format!("The {handle} room"),
                kind: EntityKind::Place,
                container: None,
            })
        };
        let created = mailbox
            .create_fixture(
                CreateWorld {
                    id: CommandId::new(),
                    owner: owner.clone(),
                    title: "Soul Stamping".into(),
                    patch: WorldPatch {
                        declarations: vec![
                            room("witness"),
                            room("council"),
                            declare("witness", "The Witness", NewController::NarrativePersona),
                            declare("council", "The Council", NewController::OperationalAgent),
                        ],
                        operations: Vec::new(),
                        evidence: Vec::new(),
                    },
                    scale_intent: WorldScaleIntentRef::default(),
                },
                &authenticated,
            )
            .await
            .unwrap();
        let mut snapshot = mailbox.snapshot().await.unwrap();
        for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
            mailbox
                .submit_fixture(
                    CommandEnvelope {
                        id: CommandId::new(),
                        world_id: created.world_id,
                        expected_revision: snapshot.revision,
                        caller: CallerId::Principal(owner.clone()),
                        body,
                    },
                    &authenticated,
                )
                .await
                .unwrap();
            snapshot = mailbox.snapshot().await.unwrap();
        }
        let pick = |mode: crate::world::ControllerMode| {
            snapshot
                .opportunities
                .iter()
                .find(|value| value.controller_mode == mode)
                .expect("an active opportunity")
                .clone()
        };
        let bound = pick(crate::world::ControllerMode::OperationalAgent);
        let unrelated = pick(crate::world::ControllerMode::NarrativePersona);

        let speak = |opportunity: &DecisionOpportunity, text: &str| DecisionInvocation {
            affordance: opportunity.affordance_ids[0],
            bindings: Vec::new(),
            proposed: Vec::new(),
            speech: Some(crate::world::Statement::new(text).unwrap()),
        };
        mailbox
            .submit_controller(
                CommandId::new(),
                &unrelated,
                speak(&unrelated, "Somebody else moves."),
            )
            .await
            .unwrap();
        let moved = mailbox.snapshot().await.unwrap();
        assert_eq!(moved.revision, bound.revision + 1);

        let receipt = mailbox
            .submit_controller(CommandId::new(), &bound, speak(&bound, "Still mine."))
            .await
            .expect("a bound turn must not die at the envelope CAS");
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
        assert_eq!(
            mailbox.snapshot().await.unwrap().revision,
            bound.revision + 2
        );
    }

    /// Soul falsification: stamping the revision inside the owner must not open
    /// a hole. A stamped body still fails closed when the world derives no such
    /// opportunity.
    #[tokio::test]
    async fn soul_a_stamped_submission_still_fails_closed_on_a_foreign_scope() {
        let (_directory, _path, mailbox, _task, authenticated, _input, created) = fixture().await;
        let mut snapshot = mailbox.snapshot().await.unwrap();
        for body in [CommandBody::ApproveDraft, CommandBody::ActivateWorld] {
            mailbox
                .submit_fixture(
                    CommandEnvelope {
                        id: CommandId::new(),
                        world_id: created.world_id,
                        expected_revision: snapshot.revision,
                        caller: CallerId::Principal(PrincipalId::new("owner")),
                        body,
                    },
                    &authenticated,
                )
                .await
                .unwrap();
            snapshot = mailbox.snapshot().await.unwrap();
        }
        assert_eq!(snapshot.phase, crate::world::WorldPhase::Active);
        let forged = DecisionOpportunity {
            world_id: WorldId::issue(),
            revision: 0,
            scope_digest: crate::world::ScopeDigest::fixture("sha256:not-a-scope"),
            scope: crate::world::DecisionScope {
                subject_id: crate::world::SubjectId::issue(),
            },
            controller_id: crate::world::ControllerId::issue(),
            controller_mode: crate::world::ControllerMode::OperationalAgent,
            affordance_ids: vec![crate::world::AffordanceId::issue()],
        };
        let result = mailbox
            .submit_controller(
                CommandId::new(),
                &forged,
                DecisionInvocation {
                    affordance: forged.affordance_ids[0],
                    bindings: Vec::new(),
                    proposed: Vec::new(),
                    speech: Some(crate::world::Statement::new("Let me in.").unwrap()),
                },
            )
            .await;
        // The owner stamps `world_id` as well as `expected_revision`, so the
        // envelope's own `WorldMismatch` check is unreachable for these two
        // bodies. `exact_opportunity` is what refuses the foreign world.
        assert!(
            matches!(
                result,
                Err(MailboxError::Kernel(KernelError::OpportunityMismatch))
            ),
            "a foreign scope was not refused: {result:?}"
        );
    }
}
