use crate::actions;
use crate::connection;
use crate::error::Error;
use crate::handles;
use crate::models;
use crate::nix;
use crate::responses;
use crate::schema;
use crate::{log_event, Event};
use crate::{JOBS_BEGIN, JOBS_BUILD, JOBS_END};

use diesel::prelude::*;
use serde_json::{json, Value};

use std::path::Path;

#[derive(Clone)]
pub struct Job {
    pub job: models::Job,
    pub evaluation: models::Evaluation,
    pub project: models::Project,
}

impl Job {
    pub async fn cancel(&self) {
        JOBS_BEGIN.cancel(self.job.id).await;
        JOBS_BUILD.cancel(self.job.id).await;
        JOBS_END.cancel(self.job.id).await;
        nix::build::BUILDS
            .abort(nix::DrvPath::new(&self.job.build_drv))
            .await;
    }

    pub async fn delete(&self) -> Result<(), Error> {
        self.cancel().await;

        let mut conn = connection().await;
        diesel::delete(schema::jobs::table.find(self.job.id)).execute(&mut *conn)?;
        diesel::delete(schema::logs::table.find(&self.job.begin_log_id)).execute(&mut *conn)?;
        diesel::delete(schema::logs::table.find(&self.job.end_log_id)).execute(&mut *conn)?;
        drop(conn);

        Ok(())
    }

    pub async fn get(handle: &handles::Job) -> Result<Self, Error> {
        let mut conn = connection().await;
        let (job, (evaluation, project)) = schema::jobs::table
            .inner_join(schema::evaluations::table.inner_join(schema::projects::table))
            .filter(schema::projects::name.eq(&handle.evaluation.project.name))
            .filter(schema::evaluations::num.eq(&handle.evaluation.num))
            .filter(schema::jobs::system.eq(&handle.system))
            .filter(schema::jobs::name.eq(&handle.name))
            .first(&mut *conn)
            .optional()?
            .ok_or(Error::JobNotFound(handle.clone()))?;
        Ok(Self {
            job,
            evaluation,
            project,
        })
    }

    pub fn handle(&self) -> handles::Job {
        handles::job((
            self.project.name.clone(),
            self.evaluation.num,
            self.job.system.clone(),
            self.job.name.clone(),
        ))
    }

    pub fn info(&self) -> responses::JobInfo {
        responses::JobInfo {
            begin_status: self.job.begin_status.clone(),
            begin_time_finished: self.job.begin_time_finished,
            begin_time_started: self.job.begin_time_started,
            build_drv: self.job.build_drv.clone(),
            build_out: self.job.build_out.clone(),
            build_status: self.job.build_status.clone(),
            build_time_finished: self.job.build_time_finished,
            build_time_started: self.job.build_time_started,
            dist: self.job.dist,
            end_status: self.job.end_status.clone(),
            end_time_finished: self.job.end_time_finished,
            end_time_started: self.job.end_time_started,
            system: self.job.system.clone(),
            time_created: self.job.time_created,
        }
    }

    pub async fn log_begin(&self) -> Result<Option<String>, Error> {
        let mut conn = connection().await;
        let stderr = schema::logs::dsl::logs
            .find(self.job.begin_log_id)
            .select(schema::logs::stderr)
            .first::<Option<String>>(&mut *conn)?;
        Ok(stderr)
    }

    pub async fn log_end(&self) -> Result<Option<String>, Error> {
        let mut conn = connection().await;
        let stderr = schema::logs::dsl::logs
            .find(self.job.end_log_id)
            .select(schema::logs::stderr)
            .first::<Option<String>>(&mut *conn)?;
        Ok(stderr)
    }

    async fn mk_input(&self, status: &str) -> Result<Value, Error> {
        Ok(json!({
            "drv": self.job.build_drv,
            "evaluation": self.evaluation.num,
            "flake": self.evaluation.flake,
            "job": self.job.name,
            "jobset": self.evaluation.jobset_name,
            "out": self.job.build_out,
            "project": self.project.name,
            "status": status,
            "system": self.job.system,
            "url": self.evaluation.url,
        }))
    }

    pub async fn run(self) -> Result<(), Error> {
        use crate::time::now;

        let drv = nix::DrvPath::new(&self.job.build_drv);

        // FIXME?
        let self_1 = self.clone();
        let self_2 = self.clone();
        let self_3 = self.clone();
        let self_4 = self.clone();
        let self_5 = self.clone();
        let self_6 = self.clone();

        // TODO: factor out common code between `begin` and `end`
        let task_begin = async move {
            let mut conn = connection().await;
            let _ = diesel::update(&self_1.job)
                .set(schema::jobs::begin_time_started.eq(now()))
                .execute(&mut *conn);
            drop(conn);

            let input = self_1.mk_input(&"pending".to_string()).await?;
            let default_log = serde_json::to_string_pretty(&input).unwrap();
            let log = if let Some(path) = self_1.evaluation.actions_path {
                if Path::new(&format!("{}/begin", path)).exists() {
                    let (_, log) = actions::run(
                        &self_1.project.key,
                        &format!("{}/begin", path),
                        &format!("{}/secrets", path),
                        &input,
                    )
                    .await?;
                    log
                } else {
                    default_log
                }
            } else {
                default_log
            };

            Ok::<_, Error>(log)
        };
        let finish_begin = move |r: Option<Result<String, Error>>| async move {
            let status = match r {
                Some(Ok(log)) => {
                    let mut conn = connection().await;
                    diesel::update(schema::logs::dsl::logs.find(self_2.job.begin_log_id))
                        .set(schema::logs::stderr.eq(log))
                        .execute(&mut *conn)
                        .unwrap(); // FIXME: no unwrap
                    "success"
                }
                Some(Err(e)) => {
                    let mut conn = connection().await;
                    diesel::update(schema::logs::dsl::logs.find(self_2.job.end_log_id))
                        .set(schema::logs::stderr.eq(e.to_string()))
                        .execute(&mut *conn)
                        .unwrap(); // FIXME: no unwrap
                    "error"
                }
                None => "canceled",
            };
            // FIXME: error management
            let mut conn = connection().await;
            let _ = diesel::update(&self_2.job)
                .set((
                    schema::jobs::begin_status.eq(status),
                    schema::jobs::begin_time_finished.eq(now()),
                ))
                .execute(&mut *conn);
            drop(conn);
            log_event(Event::JobUpdated(self_2.handle())).await;
        };
        JOBS_BEGIN.run(self.job.id, task_begin, finish_begin).await;

        let (sender, receiver) = tokio::sync::oneshot::channel::<String>();
        let task_build = async move {
            let mut conn = connection().await;
            let _ = diesel::update(&self_3.job)
                .set(schema::jobs::build_time_started.eq(now()))
                .execute(&mut *conn);
            drop(conn);
            nix::build::BUILDS.run(drv).await
        };
        let finish_build = move |r: Option<Option<Result<nix::DrvOutputs, nix::Error>>>| async move {
            let r = r.flatten();
            let status = match r {
                Some(Ok(_)) => "success",
                Some(Err(_)) => "error", // TODO: log error
                None => "canceled",
            };
            let _ = sender.send(status.to_string());
            let mut conn = connection().await;
            let _ = diesel::update(&self_4.job)
                .set((
                    schema::jobs::build_status.eq(status),
                    schema::jobs::build_time_finished.eq(now()),
                ))
                .execute(&mut *conn);
            drop(conn);
            log_event(Event::JobUpdated(self_4.handle())).await;
        };
        JOBS_BUILD.run(self.job.id, task_build, finish_build).await;

        let task_end = async move {
            // wait for `begin` to finish
            JOBS_BEGIN.wait(&self_5.job.id).await;
            // wait for the build to finish
            JOBS_BUILD.wait(&self_5.job.id).await;
            let build_status = receiver.await.unwrap_or_else(|_| panic!());

            let mut conn = connection().await;
            diesel::update(&self_5.job)
                .set(schema::jobs::end_time_started.eq(now()))
                .execute(&mut *conn)?;
            drop(conn);

            let input = self_5.mk_input(&build_status).await?;
            let default_log = serde_json::to_string_pretty(&input).unwrap();
            let log = if let Some(path) = self_5.evaluation.actions_path {
                if Path::new(&format!("{}/end", path)).exists() {
                    let (_, log) = actions::run(
                        &self_5.project.key,
                        &format!("{}/end", path),
                        &format!("{}/secrets", path),
                        &input,
                    )
                    .await?;
                    log
                } else {
                    default_log
                }
            } else {
                default_log
            };

            Ok::<_, Error>(log)
        };
        let finish_end = move |r: Option<Result<String, Error>>| async move {
            let status = match r {
                Some(Ok(log)) => {
                    let mut conn = connection().await;
                    diesel::update(schema::logs::dsl::logs.find(self_6.job.end_log_id))
                        .set(schema::logs::stderr.eq(log))
                        .execute(&mut *conn)
                        .unwrap(); // FIXME: no unwrap
                    "success"
                }
                Some(Err(e)) => {
                    let mut conn = connection().await;
                    diesel::update(schema::logs::dsl::logs.find(self_6.job.end_log_id))
                        .set(schema::logs::stderr.eq(e.to_string()))
                        .execute(&mut *conn)
                        .unwrap(); // FIXME: no unwrap
                    "error"
                }
                None => "canceled",
            };
            let mut conn = connection().await;
            // FIXME: error management
            let _ = diesel::update(&self_6.job)
                .set((
                    schema::jobs::end_status.eq(status),
                    schema::jobs::end_time_finished.eq(now()),
                ))
                .execute(&mut *conn);
            drop(conn);
            log_event(Event::JobUpdated(self_6.handle())).await;
        };
        JOBS_END.run(self.job.id, task_end, finish_end).await;

        Ok(())
    }
}
