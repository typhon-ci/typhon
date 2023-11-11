use std::collections::HashMap;
use std::future::Future;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::Mutex;

#[derive(Debug)]
pub enum Error {
    ShuttingDown,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Task manager is shutting down")
    }
}

enum Msg<Id, St: 'static> {
    Cancel(Id),
    Finish(Id),
    Run(
        Id,
        oneshot::Sender<mpsc::Sender<()>>,
        oneshot::Sender<()>,
        oneshot::Sender<&'static St>,
    ),
    Shutdown,
    Wait(Id, oneshot::Sender<()>),
}

struct TaskHandle {
    canceler: Option<oneshot::Sender<()>>,
    waiters: Vec<oneshot::Sender<()>>,
}

pub struct TaskManager<Id, St: 'static> {
    msg_send: mpsc::Sender<Msg<Id, St>>,
    shutdown_recv: Mutex<Option<oneshot::Receiver<()>>>,
}

impl<
        Id: std::cmp::Eq + std::hash::Hash + std::clone::Clone + Send + Sync + 'static,
        St: Send + Sync,
    > TaskManager<Id, St>
{
    pub fn new(state: &'static St) -> Self {
        let (msg_send, mut msg_recv) = mpsc::channel(256);
        let (shutdown_send, shutdown_recv) = oneshot::channel();
        tokio::spawn(async move {
            let _shutdown_send = shutdown_send;
            let (finish_send, mut finish_recv) = mpsc::channel(1);
            let mut tasks: HashMap<Id, TaskHandle> = HashMap::new();
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
                    (false, Msg::Run(id, finish_send_send, cancel_send, state_send)) => {
                        let _ = finish_send_send.send(finish_send.clone());
                        let _ = state_send.send(state);
                        let task = TaskHandle {
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
            drop(finish_send);
            let _ = finish_recv.recv().await;
        });
        let shutdown_recv = Mutex::new(Some(shutdown_recv));
        Self {
            msg_send,
            shutdown_recv,
        }
    }

    pub async fn wait(&self, id: &Id) -> () {
        let (sender, receiver) = oneshot::channel();
        let _ = self.msg_send.send(Msg::Wait(id.clone(), sender)).await;
        let _ = receiver.await;
    }

    // TODO: `finish` should be able to output an error
    pub fn run<
        T: Send + 'static,
        O: Future<Output = T> + Send + 'static,
        F: (FnOnce(Option<T>, &St) -> ()) + Send + Sync + 'static,
    >(
        &self,
        id: Id,
        run: O,
        finish: F,
    ) {
        use tokio::task::spawn_blocking;

        let (cancel_send, cancel_recv) = oneshot::channel::<()>();
        let (finish_send_send, finish_send_recv) = oneshot::channel::<mpsc::Sender<()>>();
        let (state_send, state_recv) = oneshot::channel::<&'static St>();
        let sender_self = self.msg_send.clone();
        let id_bis = id.clone();
        tokio::spawn(async move {
            let state = state_recv.await.unwrap(); // FIXME
            let r = tokio::select! {
                _ = cancel_recv => None,
                r = run => Some(r),
            };
            let _ = spawn_blocking(move || finish(r, state)).await;
            let _ = sender_self.send(Msg::Finish(id_bis)).await;
            let _ = finish_send_recv.await;
        });
        let _ = self
            .msg_send
            .try_send(Msg::Run(id, finish_send_send, cancel_send, state_send));
    }

    pub fn cancel(&self, id: Id) {
        let _ = self.msg_send.try_send(Msg::Cancel(id));
    }

    pub async fn shutdown(&'static self) {
        if let Some(shutdown_recv) = self.shutdown_recv.lock().await.take() {
            let _ = self.msg_send.send(Msg::Shutdown).await;
            let _ = shutdown_recv.await;
        }
    }
}
