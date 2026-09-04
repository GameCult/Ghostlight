use super::{
    AffordanceKind, AuthenticatedCaller, CallerId, CommandBody, CommandEnvelope, CommandId,
    CreateWorld, CreateWorldIntent, CreationReceipt, DecisionInvocation, DecisionOpportunity,
    Declaration, DraftHandle, KernelError, NewController, PrincipalCommandIntent, PrincipalId,
    SubjectDeclaration, SubjectKind, SubmitReceipt, WorldKernel, WorldPatch, WorldSnapshot,
    journal, prepare_creation,
};
use crate::app_session::VerifiedPrincipalEvidence;
use std::collections::BTreeSet;
use std::path::Path;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

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
    Snapshot {
        reply: oneshot::Sender<Result<WorldSnapshot, KernelError>>,
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
        let declare = |handle: &str, label: String, kind, controller| {
            Declaration::Subject(SubjectDeclaration {
                handle: DraftHandle::new(handle),
                label: label.trim().to_owned(),
                kind,
                controller,
                affordances: BTreeSet::from([AffordanceKind::Speak]),
                authority_scope: None,
            })
        };
        let mut declarations = vec![declare(
            "first-person",
            input.human_subject_label,
            SubjectKind::Person,
            NewController::Human {
                principal: principal_id.clone(),
            },
        )];
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
        self.submit_authenticated(
            CommandEnvelope {
                id: intent.id,
                world_id: intent.world_id,
                expected_revision: intent.expected_revision,
                caller: CallerId::Principal(principal_id.clone()),
                body: intent.body,
            },
            AuthenticatedCaller::verified_principal(principal_id),
        )
        .await
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
        self.submit_authenticated(
            CommandEnvelope {
                id: command_id,
                world_id: opportunity.world_id,
                expected_revision: opportunity.revision,
                caller: CallerId::Controller(controller_id),
                body: CommandBody::ExerciseDecision {
                    opportunity: opportunity.clone(),
                    invocation,
                },
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
        self.submit_authenticated(
            CommandEnvelope {
                id: command_id,
                world_id: opportunity.world_id,
                expected_revision: opportunity.revision,
                caller: CallerId::Controller(controller_id),
                body: CommandBody::DeclineDecision {
                    opportunity: opportunity.clone(),
                },
            },
            AuthenticatedCaller::verified_controller(controller_id),
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
        AffordanceKind, CallerId, CommandBody, Declaration, DraftHandle, Mismatch, NewController,
        PrincipalId, SubjectDeclaration, SubjectKind, WorldId,
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
                declarations: vec![Declaration::Subject(SubjectDeclaration {
                    handle: DraftHandle::new("operator"),
                    label: "Operator".into(),
                    kind: SubjectKind::Person,
                    controller: NewController::Human {
                        principal: PrincipalId::new("owner"),
                    },
                    affordances: BTreeSet::from([AffordanceKind::Speak]),
                    authority_scope: None,
                })],
                operations: Vec::new(),
                evidence: Vec::new(),
            },
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
            .push(invalid.patch.declarations[0].clone());
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
}
