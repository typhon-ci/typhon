use tokio::sync::oneshot::{channel, Sender};
use tokio::sync::Mutex;

use std::collections::HashMap;
use std::future::Future;

#[derive(Debug)]
struct TaskHandle {
    canceler: Sender<()>,
    waiters: Vec<Sender<()>>,
}

#[derive(Debug)]
pub struct Tasks<Id> {
    tasks: Mutex<HashMap<Id, TaskHandle>>,
}

impl<Id: std::cmp::Eq + std::hash::Hash + std::clone::Clone + Send> Tasks<Id> {
    pub fn new() -> Self {
        Tasks {
            tasks: Mutex::new(HashMap::new()),
        }
    }

    pub async fn wait(&self, id: &Id) -> () {
        let mut tasks = self.tasks.lock().await;
        let (send, recv) = channel::<()>();
        match tasks.get_mut(&id) {
            Some(task) => {
                task.waiters.push(send);
            }
            None => {
                let _ = send.send(());
            }
        }
        drop(tasks);
        let _ = recv.await;
    }

    pub async fn is_running(&self, id: &Id) -> bool {
        let tasks = self.tasks.lock().await;
        tasks.get(&id).is_some()
    }

    pub async fn run<
        S: Send + 'static,
        T: Future<Output = S> + Send + 'static,
        U: Future<Output = ()> + Send + 'static,
        F: FnOnce(Option<S>) -> U + Send + 'static,
    >(
        &'static self,
        id: Id,
        task: T,
        f: F,
    ) -> () {
        let (send, recv) = channel::<()>();
        let handle = TaskHandle {
            canceler: send,
            waiters: Vec::new(),
        };
        let mut m = self.tasks.lock().await;
        m.insert(id.clone(), handle);
        drop(m);
        tokio::spawn(async move {
            tokio::select! {
                _ = recv => {
                    f(None).await
                },
                r = task => {
                    self.tasks.lock().await.remove(&id).map(|task| {
                        for send in task.waiters {
                            let _ = send.send(());
                        }
                    });
                    f(Some(r)).await
                },
            }
        });
    }

    pub async fn cancel(&self, id: Id) -> bool {
        let mut tasks = self.tasks.lock().await;
        tasks
            .remove(&id)
            .map(|task| {
                for send in task.waiters {
                    let _ = send.send(());
                }
                let _ = task.canceler.send(());
                true
            })
            .unwrap_or(false)
    }
}
