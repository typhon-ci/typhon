use crate::error::Error;
use crate::models;
use crate::schema;
use crate::Conn;
use crate::DbPool;
use crate::{LOGS, TASKS};

use typhon_types::data::TaskStatusKind;
use typhon_types::responses::TaskStatus;

use diesel::prelude::*;
use std::future::Future;
use time::OffsetDateTime;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct Task {
    pub task: models::Task,
}

impl Task {
    pub fn cancel(&self) {
        TASKS.cancel(self.task.id);
    }

    pub fn log(&self, conn: &mut Conn) -> Result<Option<String>, Error> {
        let stderr = schema::logs::dsl::logs
            .find(self.task.log_id)
            .select(schema::logs::stderr)
            .first::<Option<String>>(conn)?;
        Ok(stderr)
    }

    pub fn new(conn: &mut Conn) -> Result<Self, Error> {
        let log = diesel::insert_into(schema::logs::dsl::logs)
            .values(models::NewLog { stderr: None })
            .get_result::<models::Log>(conn)?;
        let new_task = models::NewTask {
            log_id: log.id,
            status: TaskStatusKind::Pending.to_i32(),
        };
        let task = diesel::insert_into(schema::tasks::dsl::tasks)
            .values(new_task)
            .get_result::<models::Task>(conn)?;
        Ok(Task { task })
    }

    pub fn run<
        T: Send + 'static,
        O: Future<Output = T> + Send + 'static,
        F: (FnOnce(mpsc::Sender<String>) -> O) + Send + 'static,
        G: (FnOnce(Option<T>, &DbPool) -> TaskStatusKind) + Send + Sync + 'static,
    >(
        &self,
        conn: &mut Conn,
        run: F,
        finish: G,
    ) -> Result<(), Error> {
        let time_started = OffsetDateTime::now_utc();

        let _ = self.set_status(conn, TaskStatus::Pending(Some(time_started)));

        let id = self.task.id;
        let task_1 = self.clone();

        let (sender, mut receiver) = mpsc::channel(256);
        let run = async move {
            let (res, ()) = tokio::join!(run(sender), async move {
                while let Some(line) = receiver.recv().await {
                    LOGS.send_line(&id, line);
                }
            },);
            res
        };
        let finish = move |res: Option<T>, pool: &DbPool| {
            let mut conn = pool.get().unwrap();
            let status_kind = finish(res, pool);
            let time_finished = OffsetDateTime::now_utc();
            let stderr = LOGS.dump(&id).unwrap_or(String::new()); // FIXME
            LOGS.reset(&id);
            let status = match status_kind {
                TaskStatusKind::Pending => TaskStatus::Pending(Some(time_started)), // should not happen
                TaskStatusKind::Success => TaskStatus::Success(time_started, time_finished),
                TaskStatusKind::Error => TaskStatus::Error(time_started, time_finished),
                TaskStatusKind::Canceled => {
                    TaskStatus::Canceled(Some((time_started, time_finished)))
                }
            };
            let _ = task_1.set_status(&mut conn, status);
            let _ =
                diesel::update(schema::logs::table.filter(schema::logs::id.eq(task_1.task.log_id)))
                    .set(schema::logs::stderr.eq(stderr))
                    .execute(&mut conn);
        };

        TASKS.run(id, run, finish);

        Ok(())
    }

    pub fn status(&self) -> TaskStatus {
        TaskStatus::from_data(
            self.task.status,
            self.task.time_started,
            self.task.time_finished,
        )
    }

    fn set_status(&self, conn: &mut Conn, status: TaskStatus) -> Result<(), Error> {
        let (kind, started, finished) = status.to_data();
        let _ = diesel::update(&self.task)
            .set((
                schema::tasks::status.eq(kind),
                schema::tasks::time_started.eq(started),
                schema::tasks::time_finished.eq(finished),
            ))
            .execute(conn)?;
        Ok(())
    }
}
