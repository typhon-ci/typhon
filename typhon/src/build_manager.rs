use crate::nix;
use crate::nix::DrvPath;
use crate::task_manager::TaskManager;

use once_cell::sync::Lazy;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use std::collections::HashMap;

type Output = Result<nix::DrvOutputs, nix::Error>;

enum Msg {
    Abort(DrvPath),
    Build(DrvPath, oneshot::Sender<Option<Output>>),
    Finished(DrvPath, Option<Output>),
    Shutdown,
}

pub struct Builder {
    handle: Mutex<Option<JoinHandle<()>>>,
    sender: mpsc::Sender<Msg>,
}

impl Builder {
    fn new() -> Self {
        let (sender, mut receiver) = mpsc::channel(256);
        let sender_self = sender.clone();
        let handle = tokio::spawn(async move {
            let mut waiters: HashMap<DrvPath, (Vec<oneshot::Sender<Option<Output>>>, usize)> =
                HashMap::new();
            while let Some(msg) = receiver.recv().await {
                match msg {
                    Msg::Abort(drv) => {
                        if let Some(waiters) = waiters.get_mut(&drv) {
                            waiters.1 = waiters.1 - 1;
                            if waiters.1 == 0 {
                                TASKS.cancel(drv).await;
                            }
                        }
                    }
                    Msg::Build(drv, sender) => {
                        if let Some((senders, n)) = waiters.get_mut(&drv) {
                            senders.push(sender);
                            *n = *n + 1;
                        } else {
                            waiters.insert(drv.clone(), (vec![sender], 1));
                            let sender_self_1 = sender_self.clone();
                            let sender_self_2 = sender_self.clone();
                            let drv_1 = drv.clone();
                            let drv_2 = drv.clone();
                            let run = async move {
                                if nix::is_cached(&drv_1).await == Ok(false) {
                                    let json: serde_json::Value =
                                        nix::derivation_json(&nix::Expr::Path(drv_1.to_string()))
                                            .await?;
                                    let input_drvs =
                                        json[&drv_1.to_string()]["inputDrvs"].as_object().unwrap();
                                    let mut receivers: Vec<oneshot::Receiver<Option<Output>>> =
                                        Vec::new();
                                    for (drv, _) in input_drvs {
                                        let (sender, receiver) = oneshot::channel();
                                        let _ = sender_self_1
                                            .send(Msg::Build(DrvPath::new(drv), sender))
                                            .await;
                                        receivers.push(receiver);
                                    }
                                    for receiver in receivers.drain(..) {
                                        let _ = receiver.await;
                                    }
                                }
                                nix::build(&drv_1).await
                            };
                            let finish = |res: Option<Output>| async move {
                                let _ = sender_self_2.send(Msg::Finished(drv_2, res.clone())).await;
                            };
                            let _ = TASKS.run(drv, run, finish).await;
                        }
                    }
                    Msg::Finished(drv, res) => {
                        if let Some((senders, _)) = waiters.remove(&drv) {
                            for sender in senders {
                                let _ = sender.send(res.clone());
                            }
                        }
                    }
                    Msg::Shutdown => break,
                }
            }
        });
        let handle = Mutex::new(Some(handle));
        Self { handle, sender }
    }

    pub async fn run(&self, drv: DrvPath) -> Option<Output> {
        let (sender, receiver) = oneshot::channel();
        let _ = self.sender.send(Msg::Build(drv, sender)).await;
        if let Ok(res) = receiver.await {
            res
        } else {
            None
        }
    }

    pub async fn abort(&self, drv: DrvPath) {
        let _ = self.sender.send(Msg::Abort(drv)).await;
    }

    pub async fn shutdown(&self) {
        let handle = self.handle.lock().await.take();
        if let Some(handle) = handle {
            if self.sender.send(Msg::Shutdown).await.is_ok() {
                let _ = handle.await;
            } else {
                handle.abort();
            }
        }
        TASKS.shutdown().await;
    }
}

static TASKS: Lazy<TaskManager<DrvPath>> = Lazy::new(TaskManager::new);
pub static BUILDS: Lazy<Builder> = Lazy::new(Builder::new);
