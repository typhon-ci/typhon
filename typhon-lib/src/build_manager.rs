use crate::builds;
use crate::error::Error;
use crate::log_event;
use crate::models;
use crate::nix;
use crate::nix::DrvPath;
use crate::schema;
use crate::tasks;
use crate::Conn;
use crate::DbPool;
use crate::POOL;
use crate::RUNTIME;

use typhon_types::{data::TaskStatusKind, *};

use diesel::prelude::*;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use time::OffsetDateTime;
use tokio::{
    sync::{mpsc, oneshot, watch},
    task::JoinSet,
};

type Output = Option<Option<()>>;

pub struct BuildHandle {
    pub abort: oneshot::Sender<()>,
    pub id: i32,
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
    build: builds::Build,
    senders: Vec<oneshot::Sender<Output>>,
    active_waiters: usize,
}

struct State {
    conn: Conn,
    builds: HashMap<DrvPath, Build>,
    join_set: JoinSet<()>,
}

impl State {
    fn new() -> Self {
        Self {
            conn: POOL.get().unwrap(),
            builds: HashMap::new(),
            join_set: JoinSet::new(),
        }
    }

    async fn new_build(
        &mut self,
        drv: DrvPath,
        sender: &mpsc::UnboundedSender<Msg>,
        abort_receiver: oneshot::Receiver<()>,
        res_sender: oneshot::Sender<Output>,
    ) -> Result<i32, Error> {
        let (build, task) = self
            .conn
            .transaction::<(models::Build, tasks::Task), Error, _>(|conn| {
                let max = schema::builds::table
                    .filter(schema::builds::drv.eq(drv.to_string()))
                    .select(diesel::dsl::max(schema::builds::num))
                    .first::<Option<i64>>(conn)?
                    .unwrap_or(0);
                let num = max + 1;
                let task = tasks::Task::new(conn)?;
                let new_build = models::NewBuild {
                    drv: &drv.to_string(),
                    num,
                    task_id: task.task.id,
                    time_created: OffsetDateTime::now_utc().unix_timestamp(),
                };
                let build = diesel::insert_into(schema::builds::table)
                    .values(&new_build)
                    .get_result::<models::Build>(conn)?;

                Ok((build, task))
            })?;
        let build = builds::Build { build, task };

        self.builds.insert(
            drv.clone(),
            Build {
                build: build.clone(),
                senders: vec![res_sender],
                active_waiters: 1,
            },
        );

        log_event(Event::BuildNew(build.handle()));

        let abort = {
            let drv = drv.clone();
            let sender = sender.clone();
            abort_thread(drv, sender, abort_receiver)
        };
        self.join_set.spawn(abort);

        let run = {
            let drv = drv.clone();
            let sender = sender.clone();
            move |sender_log| run_build(drv, sender, sender_log)
        };
        let finish = {
            let drv = drv.clone();
            let handle = build.handle();
            let sender = sender.clone();
            |res, _: &DbPool| {
                let status = finish_build(drv, sender, res);
                (status, Event::BuildFinished(handle))
            }
        };
        build.task.run(&mut self.conn, run, finish)?;

        Ok(build.build.id)
    }
}

fn finish_build(drv: DrvPath, sender: mpsc::UnboundedSender<Msg>, res: Output) -> TaskStatusKind {
    let _ = sender.send(Msg::Finished(drv, res.clone()));
    match res {
        Some(Some(())) => TaskStatusKind::Success,
        Some(None) => TaskStatusKind::Error,
        None => TaskStatusKind::Canceled,
    }
}

async fn run_build(
    drv: DrvPath,
    sender: mpsc::UnboundedSender<Msg>,
    sender_log: mpsc::UnboundedSender<String>,
) -> Option<()> {
    if nix::is_cached(&drv).await == Ok(false) {
        let json: serde_json::Value = nix::derivation_json(&nix::Expr::Path(drv.to_string()))
            .await
            .ok()?;
        let input_drvs = json[&drv.to_string()]["inputDrvs"].as_object().unwrap();
        let mut handle_receivers: Vec<oneshot::Receiver<BuildHandle>> = Vec::new();
        for (drv, _) in input_drvs {
            let (handle_sender, handle_receiver) = oneshot::channel();
            let _ = sender.send(Msg::Build(DrvPath::new(drv), handle_sender));
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
    let _ = nix::build(&drv, sender_log).await.ok()?;
    Some(())
}

async fn abort_thread(
    drv: DrvPath,
    sender: mpsc::UnboundedSender<Msg>,
    receiver: oneshot::Receiver<()>,
) {
    let _ = receiver.await;
    let _ = sender.send(Msg::Abort(drv));
}

async fn main_thread(
    sender: mpsc::UnboundedSender<Msg>,
    mut receiver: mpsc::UnboundedReceiver<Msg>,
) -> Result<(), Error> {
    let mut state = State::new();
    while let Some(msg) = receiver.recv().await {
        match msg {
            Msg::Abort(drv) => {
                if let Some(build) = state.builds.get_mut(&drv) {
                    build.active_waiters = build.active_waiters - 1;
                    if build.active_waiters == 0 {
                        build.build.task.cancel();
                    }
                }
            }
            Msg::Build(drv, handle_sender) => {
                let (abort_sender, abort_receiver) = oneshot::channel();
                let (res_sender, res_receiver) = oneshot::channel();
                let id = if let Some(build) = state.builds.get_mut(&drv) {
                    build.senders.push(res_sender);
                    build.active_waiters = build.active_waiters + 1;
                    build.build.build.id
                } else {
                    let maybe_build: Option<builds::Build> =
                        builds::Build::last(&mut state.conn, &drv)?;
                    match maybe_build {
                        Some(build) => {
                            if TaskStatusKind::from(&build.task.status()) == TaskStatusKind::Success
                                && nix::is_built(&drv).await?
                            {
                                let _ = res_sender.send(Some(Some(())));
                                build.build.id
                            } else {
                                state
                                    .new_build(drv, &sender, abort_receiver, res_sender)
                                    .await?
                            }
                        }
                        None => {
                            state
                                .new_build(drv, &sender, abort_receiver, res_sender)
                                .await?
                        }
                    }
                };
                let handle = BuildHandle {
                    abort: abort_sender,
                    id,
                    wait: res_receiver,
                };
                let _ = handle_sender.send(handle);
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
    for (_, build) in state.builds {
        build.build.task.cancel();
        for sender in build.senders {
            let _ = sender.send(None);
        }
    }
    Ok(())
}

pub struct Builder {
    sender: mpsc::UnboundedSender<Msg>,
    watch: watch::Receiver<()>,
}

impl Builder {
    fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let (watch_send, watch) = watch::channel(());
        {
            let sender = sender.clone();
            RUNTIME.spawn(async move {
                let res = main_thread(sender, receiver).await;
                if let Err(e) = res {
                    log::error!("Build manager's main thread raised an error: {}", e);
                }
                let _watch_send = watch_send;
            });
        }
        Self { sender, watch }
    }

    pub fn run(&self, drv: DrvPath) -> BuildHandle {
        let (handle_sender, handle_receiver) = oneshot::channel();
        self.sender.send(Msg::Build(drv, handle_sender)).unwrap(); // FIXME
        handle_receiver.blocking_recv().unwrap() // FIXME
    }

    pub async fn shutdown(&self) {
        let _ = self.sender.send(Msg::Shutdown);
        while self.watch.clone().changed().await.is_ok() {}
    }
}

pub static BUILDS: Lazy<Builder> = Lazy::new(Builder::new);
