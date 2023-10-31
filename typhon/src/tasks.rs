use crate::connection;
use crate::error::Error;
use crate::models;
use crate::schema;
use crate::time::now;
use crate::{LOGS, TASKS};

use typhon_types::data::TaskStatusKind;
use typhon_types::responses::TaskStatus;

use diesel::prelude::*;
use std::future::Future;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct Task {
    pub task: models::Task,
}

impl Task {
    pub async fn cancel(&self) {
        TASKS.cancel(self.task.id).await;
    }

    pub async fn log(&self) -> Result<Option<String>, Error> {
        let mut conn = connection().await;
        let stderr = schema::logs::dsl::logs
            .find(self.task.log_id)
            .select(schema::logs::stderr)
            .first::<Option<String>>(&mut *conn)?;
        Ok(stderr)
    }

    pub fn new(conn: &mut SqliteConnection) -> Result<Self, Error> {
        let log = diesel::insert_into(schema::logs::dsl::logs)
            .values(models::NewLog { stderr: None })
            .get_result::<models::Log>(&mut *conn)?;
        let new_task = models::NewTask {
            log_id: log.id,
            status: TaskStatusKind::Pending.to_i32(),
        };
        let task = diesel::insert_into(schema::tasks::dsl::tasks)
            .values(new_task)
            .get_result::<models::Task>(&mut *conn)?;
        Ok(Task { task })
    }

    pub async fn run<
        T: Send + 'static,
        O: Future<Output = T> + Send + 'static,
        F: (FnOnce(mpsc::Sender<String>) -> O) + Send + 'static,
        U: Future<Output = TaskStatusKind> + Send + 'static,
        G: (FnOnce(Option<T>) -> U) + Send + Sync + 'static,
    >(
        &self,
        run: F,
        finish: G,
    ) -> Result<(), Error> {
        use tokio_stream::StreamExt;

        let time_started = now();

        let mut conn = connection().await;
        let _ = self.set_status(&mut *conn, TaskStatus::Pending(Some(time_started)));
        drop(conn);

        let id = self.task.id;
        let task_1 = self.clone();

        let (sender, mut receiver) = mpsc::channel(256);
        let run = async move {
            let (res, ()) = tokio::join!(run(sender), async move {
                while let Some(line) = receiver.recv().await {
                    LOGS.send_line(&id, line).await;
                }
            },);
            res
        };
        let finish = move |res: Option<T>| async move {
            let status_kind = finish(res).await;
            let time_finished = now();
            let maybe_stream = LOGS.listen(&id).await;
            LOGS.reset(&id).await;
            let stderr = if let Some(stream) = maybe_stream {
                stream.collect::<Vec<String>>().await.join("\n")
            } else {
                // should not happen
                String::new()
            };
            let status = match status_kind {
                TaskStatusKind::Pending => TaskStatus::Pending(Some(time_started)), // should not happen
                TaskStatusKind::Success => TaskStatus::Success(time_started, time_finished),
                TaskStatusKind::Error => TaskStatus::Error(time_started, time_finished),
                TaskStatusKind::Canceled => {
                    TaskStatus::Canceled(Some((time_started, time_finished)))
                }
            };
            let mut conn = connection().await;
            let _ = task_1.set_status(&mut *conn, status);
            let _ =
                diesel::update(schema::logs::table.filter(schema::logs::id.eq(task_1.task.log_id)))
                    .set(schema::logs::stderr.eq(stderr))
                    .execute(&mut *conn);
        };

        TASKS.run(id, run, finish).await;

        Ok(())
    }

    pub fn status(&self) -> TaskStatus {
        TaskStatus::from_data(
            self.task.status,
            self.task.time_started,
            self.task.time_finished,
        )
    }

    fn set_status(&self, conn: &mut SqliteConnection, status: TaskStatus) -> Result<(), Error> {
        let (kind, started, finished) = status.to_data();
        let _ = diesel::update(&self.task)
            .set((
                schema::tasks::status.eq(kind),
                schema::tasks::time_started.eq(started),
                schema::tasks::time_finished.eq(finished),
            ))
            .execute(&mut *conn)?;
        Ok(())
    }
}
