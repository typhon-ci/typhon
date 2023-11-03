use crate::error::Error;
use crate::nix;
use crate::nix::DrvPath;
use crate::task_manager::TaskManager;

use once_cell::sync::Lazy;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::task::JoinSet;

use std::collections::HashMap;

type Output = Option<Option<()>>;

pub struct BuildHandle {
    pub abort: oneshot::Sender<()>,
    pub wait: oneshot::Receiver<Output>,
}

impl BuildHandle {
    pub async fn wait(self) -> Output {
        let _abort = self.abort;
        self.wait.await.unwrap() // FIXME
    }
}

enum Msg {
    Abort(DrvPath),
    Build(DrvPath, oneshot::Sender<BuildHandle>),
    Finished(DrvPath, Output),
    Shutdown,
}

struct Build {
    senders: Vec<oneshot::Sender<Output>>,
    active_waiters: usize,
}

struct State {
    pub builds: HashMap<DrvPath, Build>,
    pub join_set: JoinSet<()>,
}

impl State {
    fn new() -> Self {
        Self {
            builds: HashMap::new(),
            join_set: JoinSet::new(),
        }
    }

    async fn new_build(
        &mut self,
        drv: DrvPath,
        sender: &mpsc::Sender<Msg>,
        abort_receiver: oneshot::Receiver<()>,
        res_sender: oneshot::Sender<Output>,
    ) {
        self.builds.insert(
            drv.clone(),
            Build {
                senders: vec![res_sender],
                active_waiters: 1,
            },
        );

        let abort = {
            let drv = drv.clone();
            let sender = sender.clone();
            abort_thread(drv, sender, abort_receiver)
        };
        self.join_set.spawn(abort);

        let run = {
            let drv = drv.clone();
            let sender = sender.clone();
            run_build(drv, sender)
        };
        let finish = {
            let drv = drv.clone();
            let sender = sender.clone();
            |res| finish_build(drv, sender, res)
        };
        let _ = TASKS.run(drv, run, finish).await;
    }
}

async fn finish_build(drv: DrvPath, sender: mpsc::Sender<Msg>, res: Output) {
    let _ = sender.send(Msg::Finished(drv, res)).await;
}

async fn run_build(drv: DrvPath, sender: mpsc::Sender<Msg>) -> Option<()> {
    if nix::is_cached(&drv).await == Ok(false) {
        let json: serde_json::Value = nix::derivation_json(&nix::Expr::Path(drv.to_string()))
            .await
            .ok()?;
        let input_drvs = json[&drv.to_string()]["inputDrvs"].as_object().unwrap();
        let mut handle_receivers: Vec<oneshot::Receiver<BuildHandle>> = Vec::new();
        for (drv, _) in input_drvs {
            let (handle_sender, handle_receiver) = oneshot::channel();
            let _ = sender
                .send(Msg::Build(DrvPath::new(drv), handle_sender))
                .await;
            handle_receivers.push(handle_receiver);
        }
        let mut join_set = JoinSet::new();
        for handle_receiver in handle_receivers.drain(..) {
            join_set.spawn(async move {
                let _ = handle_receiver.await.unwrap().wait().await; // FIXME
            });
        }
        while let Some(res) = join_set.join_next().await {
            if res.is_err() {
                return None;
            }
        }
    }
    let _ = nix::build(&drv).await.ok()?;
    Some(())
}

async fn abort_thread(drv: DrvPath, sender: mpsc::Sender<Msg>, receiver: oneshot::Receiver<()>) {
    let _ = receiver.await;
    let _ = sender.send(Msg::Abort(drv));
}

async fn main_thread(
    sender: mpsc::Sender<Msg>,
    mut receiver: mpsc::Receiver<Msg>,
) -> Result<(), Error> {
    let mut state = State::new();
    while let Some(msg) = receiver.recv().await {
        match msg {
            Msg::Abort(drv) => {
                if let Some(build) = state.builds.get_mut(&drv) {
                    build.active_waiters = build.active_waiters - 1;
                    if build.active_waiters == 0 {
                        TASKS.cancel(drv).await;
                    }
                }
            }
            Msg::Build(drv, handle_sender) => {
                let (abort_sender, abort_receiver) = oneshot::channel();
                let (res_sender, res_receiver) = oneshot::channel();
                let handle = BuildHandle {
                    abort: abort_sender,
                    wait: res_receiver,
                };
                let _ = handle_sender.send(handle);
                if let Some(build) = state.builds.get_mut(&drv) {
                    build.senders.push(res_sender);
                    build.active_waiters = build.active_waiters + 1;
                } else {
                    if nix::is_built(&drv).await? {
                        let _ = res_sender.send(Some(Some(())));
                    } else {
                        state
                            .new_build(drv, &sender, abort_receiver, res_sender)
                            .await;
                    }
                }
            }
            Msg::Finished(drv, res) => {
                if let Some(build) = state.builds.remove(&drv) {
                    for sender in build.senders {
                        let _ = sender.send(res.clone());
                    }
                }
            }
            Msg::Shutdown => break,
        }
    }
    state.join_set.abort_all();
    TASKS.shutdown().await;
    for (_, build) in state.builds {
        for sender in build.senders {
            let _ = sender.send(None);
        }
    }
    Ok(())
}

pub struct Builder {
    handle: Mutex<Option<JoinHandle<()>>>,
    sender: mpsc::Sender<Msg>,
}

impl Builder {
    fn new() -> Self {
        let (sender, receiver) = mpsc::channel(256);
        let handle = {
            let sender = sender.clone();
            tokio::spawn(async move {
                let res = main_thread(sender, receiver).await;
                if let Err(e) = res {
                    log::error!("Build manager's main thread raised an error: {}", e);
                }
            })
        };
        let handle = Mutex::new(Some(handle));
        Self { handle, sender }
    }

    pub async fn run(&self, drv: DrvPath) -> BuildHandle {
        let (handle_sender, handle_receiver) = oneshot::channel();
        self.sender
            .try_send(Msg::Build(drv, handle_sender))
            .unwrap(); // FIXME
        handle_receiver.await.unwrap() // FIXME
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
    }
}

static TASKS: Lazy<TaskManager<DrvPath>> = Lazy::new(TaskManager::new);
pub static BUILDS: Lazy<Builder> = Lazy::new(Builder::new);
