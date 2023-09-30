use tokio::sync::oneshot::{channel, Sender};
use tokio::sync::Mutex;

use std::collections::HashMap;
use std::future::Future;

#[derive(Debug)]
struct TaskHandle {
    canceler: Option<Sender<()>>,
    waiters: Vec<Sender<()>>,
}

#[derive(Debug)]
struct TasksUnwrapped<Id> {
    handles: HashMap<Id, TaskHandle>,
}

#[derive(Debug)]
pub struct Tasks<Id> {
    tasks: Mutex<TasksUnwrapped<Id>>,
}

impl<Id: std::cmp::Eq + std::hash::Hash + std::clone::Clone + Send> Tasks<Id> {
    pub fn new() -> Self {
        Tasks {
            tasks: Mutex::new(TasksUnwrapped {
                handles: HashMap::new(),
            }),
        }
    }

    pub async fn wait(&self, id: &Id) -> () {
        let mut tasks = self.tasks.lock().await;
        let (send, recv) = channel::<()>();
        match tasks.handles.get_mut(&id) {
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
        tasks.handles.get(&id).is_some()
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
        let mut tasks = self.tasks.lock().await;
        let (send, recv) = channel::<()>();
        let handle = TaskHandle {
            canceler: Some(send),
            waiters: Vec::new(),
        };
        tasks.handles.insert(id.clone(), handle);
        drop(tasks);
        tokio::spawn(async move {
            let r = tokio::select! {
                _ = recv => None,
                r = task => Some(r),
            };
            f(r).await;
            self.tasks.lock().await.handles.remove(&id).map(|handle| {
                for send in handle.waiters {
                    let _ = send.send(());
                }
            });
        });
    }

    pub async fn cancel(&self, id: &Id) -> bool {
        self.tasks
            .lock()
            .await
            .handles
            .get_mut(&id)
            .map(|task| task.canceler.take().map(|send| send.send(())))
            .flatten()
            .is_some()
    }
}
