use crate::error::Error;
use crate::log_event;
use crate::models;
use crate::schema;
use crate::Conn;
use crate::POOL;
use crate::{LOGS, TASKS};

use typhon_types::data::TaskStatusKind;
use typhon_types::responses::TaskStatus;
use typhon_types::Event;

use diesel::prelude::*;
use futures_core::stream::Stream;
use std::future::Future;
use time::OffsetDateTime;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct Task {
    pub task: models::Task,
}

impl models::Task {
    pub fn status_kind(&self) -> TaskStatusKind {
        self.status.try_into().unwrap()
    }
    pub fn status(&self) -> TaskStatus {
        let from_timestamp = |t| OffsetDateTime::from_unix_timestamp(t).unwrap();
        self.status_kind().into_task_status(
            self.time_started.map(from_timestamp),
            self.time_finished.map(from_timestamp),
        )
    }
}

impl Task {
    pub fn cancel(&self) {
        TASKS.cancel(self.task.id);
    }

    pub fn log(&self, conn: &mut Conn) -> Result<Option<impl Stream<Item = String>>, Error> {
        let stream = LOGS.listen(&self.task.id);
        let stderr = schema::logs::dsl::logs
            .find(self.task.log_id)
            .select(schema::logs::stderr)
            .first::<Option<String>>(conn)?;
        Ok(Some(async_stream::stream! {
            if let Some(stream) = stream {
                for await line in stream {
                    yield line;
                }
            } else if let Some(stderr) = stderr {
                for line in stderr.split('\n') {
                    yield line.to_string();
                }
            }
        }))
    }

    pub fn new(conn: &mut Conn) -> Result<Self, Error> {
        let log = diesel::insert_into(schema::logs::dsl::logs)
            .values(models::NewLog { stderr: None })
            .get_result::<models::Log>(conn)?;
        let new_task = models::NewTask {
            log_id: log.id,
            status: TaskStatusKind::Pending.into(),
        };
        let task = diesel::insert_into(schema::tasks::dsl::tasks)
            .values(new_task)
            .get_result::<models::Task>(conn)?;
        Ok(Task { task })
    }

    pub fn run<
        T: Send + 'static,
        O: Future<Output = T> + Send + 'static,
        F: (FnOnce(mpsc::UnboundedSender<String>) -> O) + Send + 'static,
        G: (FnOnce(Option<T>) -> (TaskStatusKind, Event)) + Send + Sync + 'static,
    >(
        &self,
        conn: &mut Conn,
        run: F,
        finish: G,
    ) -> Result<(), Error> {
        let start = Some(OffsetDateTime::now_utc());
        let id = self.task.id;

        self.set_status(conn, TaskStatus::Pending { start })?;

        let (sender, mut receiver) = mpsc::unbounded_channel();
        let run = async move {
            let (res, ()) = tokio::join!(run(sender), async move {
                while let Some(line) = receiver.recv().await {
                    LOGS.send_line(&id, line);
                }
            },);
            res
        };
        let finish = {
            let task = self.clone();
            move |res: Option<T>| {
                let mut conn = POOL.get().unwrap();
                let (status_kind, event) = finish(res);
                let time_finished = OffsetDateTime::now_utc();
                let stderr = LOGS.dump(&id).unwrap_or(String::new()); // FIXME
                let status = status_kind.into_task_status(start, Some(time_finished));
                LOGS.reset(&id);
                task.set_status(&mut conn, status).unwrap();
                diesel::update(schema::logs::table.filter(schema::logs::id.eq(task.task.log_id)))
                    .set(schema::logs::stderr.eq(stderr))
                    .execute(&mut conn)
                    .unwrap(); // TODO: handle error properly
                log_event(event);
                None::<()>
            }
        };

        TASKS.run(id, (run, finish));

        Ok(())
    }

    pub fn status_kind(&self) -> TaskStatusKind {
        self.task.status_kind()
    }
    pub fn status(&self) -> TaskStatus {
        self.task.status()
    }

    fn set_status(&self, conn: &mut Conn, status: TaskStatus) -> Result<(), Error> {
        let (started, finished) = status.times();
        let _ = diesel::update(&self.task)
            .set((
                schema::tasks::status.eq(i32::from(TaskStatusKind::from(&status))),
                schema::tasks::time_started.eq(started.map(OffsetDateTime::unix_timestamp)),
                schema::tasks::time_finished.eq(finished.map(OffsetDateTime::unix_timestamp)),
            ))
            .execute(conn)?;
        Ok(())
    }
}
