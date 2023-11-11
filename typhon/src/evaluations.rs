use crate::error::Error;
use crate::jobs;
use crate::models;
use crate::nix;
use crate::responses;
use crate::schema;
use crate::tasks;
use crate::Conn;
use crate::DbPool;

use typhon_types::data::TaskStatusKind;
use typhon_types::*;

use diesel::prelude::*;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct Evaluation {
    pub task: tasks::Task,
    pub evaluation: models::Evaluation,
    pub project: models::Project,
}

impl Evaluation {
    pub fn cancel(&self) {
        self.task.cancel()
    }

    pub fn finish(
        self,
        r: Option<Result<nix::NewJobs, nix::Error>>,
        pool: &DbPool,
    ) -> TaskStatusKind {
        let mut conn = pool.get().unwrap();
        match r {
            Some(Ok(new_jobs)) => match self.create_new_jobs(&mut conn, new_jobs) {
                Ok(()) => TaskStatusKind::Success,
                Err(_) => TaskStatusKind::Error,
            },
            Some(Err(_)) => TaskStatusKind::Error,
            None => TaskStatusKind::Canceled,
        }
    }

    pub fn get(conn: &mut Conn, handle: &handles::Evaluation) -> Result<Self, Error> {
        let (evaluation, project, task) = schema::evaluations::table
            .inner_join(schema::projects::table)
            .inner_join(schema::tasks::table)
            .filter(schema::projects::name.eq(&handle.project.name))
            .filter(schema::evaluations::num.eq(handle.num as i64))
            .first(conn)
            .optional()?
            .ok_or(Error::EvaluationNotFound(handle.clone()))?;
        Ok(Self {
            task: tasks::Task { task },
            evaluation,
            project,
        })
    }

    pub fn handle(&self) -> handles::Evaluation {
        handles::evaluation((self.project.name.clone(), self.evaluation.num as u64))
    }

    pub fn info(&self, conn: &mut Conn) -> Result<responses::EvaluationInfo, Error> {
        use typhon_types::responses::JobSystemName;

        let jobs = if TaskStatusKind::from_i32(self.task.task.status) == TaskStatusKind::Success {
            let jobs = schema::jobs::table
                .filter(schema::jobs::evaluation_id.eq(self.evaluation.id))
                .load::<models::Job>(conn)?;
            Some(
                jobs.iter()
                    .map(|job| JobSystemName {
                        system: job.system.clone(),
                        name: job.name.clone(),
                    })
                    .collect(),
            )
        } else {
            None
        };
        Ok(responses::EvaluationInfo {
            actions_path: self.evaluation.actions_path.clone(),
            flake: self.evaluation.flake,
            jobs,
            jobset_name: self.evaluation.jobset_name.clone(),
            status: self.task.status(),
            time_created: time::OffsetDateTime::from_unix_timestamp(self.evaluation.time_created)?,
            url: self.evaluation.url.clone(),
        })
    }

    pub fn log(&self, conn: &mut Conn) -> Result<Option<String>, Error> {
        self.task.log(conn)
    }

    pub async fn run(self, sender: mpsc::Sender<String>) -> Result<nix::NewJobs, nix::Error> {
        let res = nix::eval_jobs(&self.evaluation.url, self.evaluation.flake).await;
        match &res {
            Err(e) => {
                for line in e.to_string().split("\n") {
                    // TODO: hide internal error messages?
                    // TODO: error management
                    let _ = sender.send(line.to_string()).await;
                }
            }
            _ => (),
        }
        res
    }

    pub fn search(
        conn: &mut Conn,
        search: &requests::EvaluationSearch,
    ) -> Result<responses::SearchResult<handles::Evaluation>, Error> {
        let query = || {
            let mut query = schema::evaluations::table
                .inner_join(schema::projects::table)
                .inner_join(
                    schema::tasks::table.on(schema::tasks::id.eq(schema::evaluations::task_id)),
                )
                .into_boxed();
            if let Some(name) = &search.project_name {
                query = query.filter(schema::projects::name.eq(name));
            }
            if let Some(name) = &search.jobset_name {
                query = query.filter(schema::evaluations::jobset_name.eq(name));
            }
            if let Some(status) = search.status {
                query = query.filter(schema::tasks::status.eq(status.to_i32()));
            }
            query.order(schema::evaluations::time_created.desc())
        };

        let (evaluations, total): (Vec<_>, i64) = conn.transaction::<_, Error, _>(|conn| {
            let total = query().count().get_result(conn)?;
            let evaluations = query()
                .offset(search.offset as i64)
                .limit(search.limit as i64)
                .load::<(models::Evaluation, models::Project, models::Task)>(conn)?;
            Ok((evaluations, total))
        })?;
        let count = evaluations.len() as u8;
        let total = total as u64;
        let list = evaluations
            .into_iter()
            .map(|(evaluation, project, _)| {
                handles::evaluation((project.name, evaluation.num as u64))
            })
            .collect();
        Ok(responses::SearchResult { count, list, total })
    }

    fn create_new_jobs(&self, conn: &mut Conn, mut new_jobs: nix::NewJobs) -> Result<(), Error> {
        let created_jobs = conn.transaction::<Vec<jobs::Job>, Error, _>(|conn| {
            new_jobs
                .drain()
                .map(|((system, name), (drv, dist))| {
                    let new_job = models::NewJob {
                        dist,
                        drv: &drv.path.to_string(),
                        evaluation_id: self.evaluation.id,
                        name: &name,
                        out: drv
                            .outputs
                            .iter()
                            .last()
                            .expect("TODO: derivations can have multiple outputs")
                            .1,
                        system: &system,
                    };
                    let job = diesel::insert_into(schema::jobs::table)
                        .values(&new_job)
                        .get_result::<models::Job>(conn)?;
                    Ok(jobs::Job {
                        project: self.project.clone(),
                        evaluation: self.evaluation.clone(),
                        job,
                    })
                })
                .collect()
        })?;

        for job in created_jobs.into_iter() {
            job.new_run(conn)?;
        }

        Ok(())
    }
}
