use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;

use std::collections::HashMap;
use std::future::Future;

#[derive(Debug)]
pub enum Error {
    ShuttingDown,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Task manager is shutting down")
    }
}

enum Msg<Id> {
    Cancel(Id),
    Finish(Id),
    Run(Id, oneshot::Sender<()>),
    Shutdown,
    Wait(Id, oneshot::Sender<()>),
}

struct TaskCtx {
    canceler: Option<oneshot::Sender<()>>,
    waiters: Vec<oneshot::Sender<()>>,
}

pub struct TaskManager<Id> {
    msg_send: mpsc::UnboundedSender<Msg<Id>>,
    watch: watch::Receiver<()>,
}

pub struct TaskHandle {
    cancel: mpsc::UnboundedSender<oneshot::Sender<()>>,
}

impl TaskHandle {
    pub async fn spawn<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        f: F,
    ) -> Option<T> {
        let (cancel_send, cancel_recv) = oneshot::channel();
        let res = tokio::spawn(async move {
            tokio::select! {
                _ = cancel_recv => None,
                r = f => Some(r),
            }
        })
        .await;
        let _ = self.cancel.send(cancel_send);
        res.unwrap()
    }
}

impl<Id: std::cmp::Eq + std::hash::Hash + std::clone::Clone + Send + Sync + 'static>
    TaskManager<Id>
{
    pub fn new() -> Self {
        let (msg_send, mut msg_recv) = mpsc::unbounded_channel();
        let (watch_send, watch) = watch::channel(());
        tokio::spawn(async move {
            let mut tasks: HashMap<Id, TaskCtx> = HashMap::new();
            let mut shutdown = false;
            while let Some(msg) = msg_recv.recv().await {
                match (shutdown, msg) {
                    (false, Msg::Cancel(id)) => {
                        let _ = tasks
                            .get_mut(&id)
                            .map(|task| task.canceler.take().map(|send| send.send(())));
                    }
                    (_, Msg::Finish(id)) => {
                        if let Some(task) = tasks.remove(&id) {
                            for send in task.waiters {
                                let _ = send.send(());
                            }
                        }
                        if shutdown && tasks.is_empty() {
                            break;
                        }
                    }
                    (false, Msg::Run(id, cancel_send)) => {
                        let task = TaskCtx {
                            canceler: Some(cancel_send),
                            waiters: Vec::new(),
                        };
                        tasks.insert(id, task);
                    }
                    (false, Msg::Shutdown) => {
                        shutdown = true;
                        let ids: Vec<_> = tasks.keys().cloned().collect();
                        for id in ids.iter() {
                            tasks
                                .get_mut(id)
                                .map(|task| task.canceler.take().map(|sender| sender.send(())));
                        }
                        if tasks.is_empty() {
                            break;
                        }
                    }
                    (_, Msg::Wait(id, sender)) => match tasks.get_mut(&id) {
                        Some(task) => {
                            task.waiters.push(sender);
                        }
                        None => {
                            let _ = sender.send(());
                        }
                    },
                    _ => (),
                }
            }
            let _watch_send = watch_send;
        });
        Self { msg_send, watch }
    }

    pub async fn wait(&self, id: &Id) -> () {
        let (sender, receiver) = oneshot::channel();
        let _ = self.msg_send.send(Msg::Wait(id.clone(), sender));
        let _ = receiver.await;
    }

    pub fn run<
        O: Future<Output = ()> + Send + 'static,
        T: FnOnce(TaskHandle) -> O + Send + 'static,
    >(
        &self,
        id: Id,
        task: T,
    ) {
        let (cancel_send, cancel_recv) = oneshot::channel::<()>();
        let sender_self = self.msg_send.clone();
        let id_bis = id.clone();

        let (cancel_thread_send, mut cancel_thread_recv) =
            mpsc::unbounded_channel::<oneshot::Sender<()>>();
        let cancel_thread = tokio::spawn(async move {
            let _ = cancel_recv.await;
            while let Some(cancel_step_send) = cancel_thread_recv.recv().await {
                let _ = cancel_step_send.send(());
            }
        });

        let handle = TaskHandle {
            cancel: cancel_thread_send,
        };
        tokio::spawn(async move {
            task(handle).await;
            cancel_thread.abort();
            let _ = cancel_thread.await;
            let _ = sender_self.send(Msg::Finish(id_bis));
        });

        let _ = self.msg_send.send(Msg::Run(id, cancel_send));
    }

    pub fn cancel(&self, id: Id) {
        let _ = self.msg_send.send(Msg::Cancel(id));
    }

    pub async fn shutdown(&'static self) {
        let _ = self.msg_send.send(Msg::Shutdown);
        while self.watch.clone().changed().await.is_ok() {}
    }
}
