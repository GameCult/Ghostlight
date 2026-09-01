use super::{
    AuthenticatedCaller, CommandEnvelope, CommandId, KernelError, SubmitReceipt, WorldKernel,
    WorldSnapshot,
};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

const REQUEST_CAPACITY: usize = 32;

#[derive(Clone, Debug)]
pub(crate) struct WorldMailbox {
    sender: mpsc::Sender<Request>,
}

enum Request {
    Submit {
        command: CommandEnvelope,
        authenticated: AuthenticatedCaller,
        reply: oneshot::Sender<Result<SubmitReceipt, KernelError>>,
    },
    Snapshot {
        reply: oneshot::Sender<Result<WorldSnapshot, KernelError>>,
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
    pub(crate) fn spawn(kernel: WorldKernel) -> (Self, JoinHandle<()>) {
        let (sender, mut receiver) = mpsc::channel(REQUEST_CAPACITY);
        let task = tokio::spawn(async move {
            let mut kernel = kernel;
            while let Some(request) = receiver.recv().await {
                match request {
                    Request::Submit {
                        command,
                        authenticated,
                        reply,
                    } => {
                        let result = kernel.submit(command, &authenticated);
                        let _ = reply.send(result);
                    }
                    Request::Snapshot { reply } => {
                        let result = kernel.snapshot();
                        let _ = reply.send(result);
                    }
                }
            }
        });
        (Self { sender }, task)
    }

    pub(crate) async fn submit(
        &self,
        command: CommandEnvelope,
        authenticated: &AuthenticatedCaller,
    ) -> Result<SubmitReceipt, MailboxError> {
        let command_id = command.id;
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Request::Submit {
                command,
                authenticated: authenticated.clone(),
                reply,
            })
            .await
            .map_err(|_| MailboxError::Unavailable)?;

        response
            .await
            .map_err(|_| MailboxError::OutcomeUnknown { command_id })?
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{
        AffordanceKind, CallerId, CommandBody, CreateWorld, DraftSubjectHandle, NewController,
        NewDecisionSubject, PrincipalId, SubjectKind,
    };
    use std::collections::BTreeSet;
    use tempfile::TempDir;

    fn fixture() -> (
        TempDir,
        WorldKernel,
        AuthenticatedCaller,
        crate::world::WorldId,
    ) {
        let directory = tempfile::tempdir().expect("temporary world directory");
        let owner = PrincipalId::new("owner");
        let authenticated = AuthenticatedCaller::fixture(CallerId::Principal(owner.clone()));
        let input = CreateWorld {
            id: CommandId::new(),
            owner,
            title: "Mailbox World".into(),
            subjects: vec![NewDecisionSubject {
                handle: DraftSubjectHandle::new("operator"),
                label: "Operator".into(),
                kind: SubjectKind::Person,
                controller: NewController::Human {
                    principal: PrincipalId::new("owner"),
                },
                affordances: BTreeSet::from([AffordanceKind::Speak]),
            }],
        };
        let (kernel, receipt) =
            WorldKernel::create(directory.path().join("world.cc"), input, &authenticated)
                .expect("create fixture world");
        (directory, kernel, authenticated, receipt.world_id)
    }

    fn title_command(
        world_id: crate::world::WorldId,
        revision: u64,
        title: &str,
    ) -> CommandEnvelope {
        CommandEnvelope {
            id: CommandId::new(),
            world_id,
            expected_revision: revision,
            caller: CallerId::Principal(PrincipalId::new("owner")),
            body: CommandBody::SetTitle {
                title: title.into(),
            },
        }
    }

    #[tokio::test]
    async fn submit_then_snapshot_share_one_serial_owner() {
        let (_directory, kernel, authenticated, world_id) = fixture();
        let (mailbox, task) = WorldMailbox::spawn(kernel);

        let receipt = mailbox
            .submit(title_command(world_id, 0, "Second Title"), &authenticated)
            .await
            .expect("submit through mailbox");
        assert!(matches!(receipt, SubmitReceipt::Applied(_)));
        let snapshot = mailbox.snapshot().await.expect("snapshot through mailbox");
        assert_eq!(snapshot.revision, 1);
        assert_eq!(snapshot.title, "Second Title");

        drop(mailbox);
        task.await.expect("owner task exits after last sender");
    }

    #[tokio::test]
    async fn concurrent_same_revision_submissions_cannot_both_commit() {
        let (_directory, kernel, authenticated, world_id) = fixture();
        let (mailbox, task) = WorldMailbox::spawn(kernel);
        let left = title_command(world_id, 0, "Left");
        let right = title_command(world_id, 0, "Right");

        let (left_result, right_result) = tokio::join!(
            mailbox.submit(left, &authenticated),
            mailbox.submit(right, &authenticated)
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
        let (directory, kernel, authenticated, world_id) = fixture();
        let (mailbox, task) = WorldMailbox::spawn(kernel);
        let (reply, response) = oneshot::channel();
        mailbox
            .sender
            .send(Request::Submit {
                command: title_command(world_id, 0, "Committed Without Listener"),
                authenticated,
                reply,
            })
            .await
            .expect("enqueue command");
        drop(response);
        drop(mailbox);

        task.await.expect("owner drains queue before exit");
        let reopened = WorldKernel::open(directory.path().join("world.cc"), world_id)
            .expect("reopen released world");
        let snapshot = reopened.snapshot().expect("read durable state");
        assert_eq!(snapshot.revision, 1);
        assert_eq!(snapshot.title, "Committed Without Listener");
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
            .submit(
                title_command(crate::world::WorldId::issue(), 0, "Uncertain"),
                &AuthenticatedCaller::fixture(CallerId::Principal(PrincipalId::new("owner"))),
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
            .submit(
                title_command(crate::world::WorldId::issue(), 0, "Never Enqueued"),
                &AuthenticatedCaller::fixture(CallerId::Principal(PrincipalId::new("owner"))),
            )
            .await;
        assert!(matches!(result, Err(MailboxError::Unavailable)));
    }
}
