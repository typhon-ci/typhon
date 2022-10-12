use std::sync::Mutex;
use tokio::sync::oneshot::{channel, Sender};

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
        let mut tasks = self.tasks.lock().unwrap();
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

    pub fn is_running(&self, id: &Id) -> bool {
        let tasks = self.tasks.lock().unwrap();
        tasks.get(&id).is_some()
    }

    pub fn run<
        S,
        T: Future<Output = S> + Send + 'static,
        F: FnOnce(Option<S>) -> () + Send + 'static,
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
        let mut m = self.tasks.lock().unwrap();
        m.insert(id.clone(), handle);
        drop(m);
        tokio::spawn(async move {
            tokio::select! {
                _ = recv => {
                    f(None)
                },
                r = task => {
                    self.tasks.lock().unwrap().remove(&id).map(|task| {
                        for send in task.waiters {
                            let _ = send.send(());
                        }
                    });
                    f(Some(r))
                },
            }
        });
    }

    pub fn cancel(&self, id: Id) -> bool {
        let mut tasks = self.tasks.lock().unwrap();
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
