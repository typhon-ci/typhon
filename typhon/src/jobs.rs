use crate::actions;
use crate::connection;
use crate::error::Error;
use crate::handles;
use crate::models::*;
use crate::nix;
use crate::responses;
use crate::schema::evaluations::dsl::*;
use crate::schema::jobs::dsl::*;
use crate::{log_event, Event};
use crate::{JOBS_BEGIN, JOBS_BUILD, JOBS_END};
use diesel::prelude::*;
use serde_json::{json, Value};
use std::path::Path;

impl Job {
    pub async fn cancel(&self) -> Result<(), Error> {
        let a = JOBS_BEGIN.cancel(&self.job_id).await;
        let b = JOBS_BUILD.cancel(&self.job_id).await;
        let c = JOBS_END.cancel(&self.job_id).await;
        if a || b || c {
            Ok(())
        } else {
            Err(Error::JobNotRunning(self.handle().await?))
        }
    }

    pub async fn evaluation(&self) -> Result<Evaluation, Error> {
        let mut conn = connection().await;
        Ok(evaluations
            .find(self.job_evaluation)
            .first::<Evaluation>(&mut *conn)?)
    }

    pub async fn get(job_handle: &handles::Job) -> Result<Self, Error> {
        let handles::pattern!(
            project_name_,
            jobset_name_,
            evaluation_num_,
            job_system_,
            job_name_
        ) = job_handle;
        let evaluation = Evaluation::get(&job_handle.evaluation).await?;
        let mut conn = connection().await;
        Ok(jobs
            .filter(job_evaluation.eq(evaluation.evaluation_id))
            .filter(job_system.eq(job_system_))
            .filter(job_name.eq(job_name_))
            .first::<Job>(&mut *conn)
            .map_err(|_| {
                Error::JobNotFound(handles::job((
                    project_name_.clone(),
                    jobset_name_.clone(),
                    *evaluation_num_,
                    job_system_.clone(),
                    job_name_.clone(),
                )))
            })?)
    }

    pub async fn handle(&self) -> Result<handles::Job, Error> {
        Ok(handles::Job {
            evaluation: self.evaluation().await?.handle().await?,
            system: self.job_system.clone(),
            name: self.job_name.clone(),
        })
    }

    pub fn info(&self) -> responses::JobInfo {
        responses::JobInfo {
            begin_status: self.job_begin_status.clone(),
            begin_time_finished: self.job_begin_time_finished,
            begin_time_started: self.job_begin_time_started,
            build_drv: self.job_build_drv.clone(),
            build_out: self.job_build_out.clone(),
            build_status: self.job_build_status.clone(),
            build_time_finished: self.job_build_time_finished,
            build_time_started: self.job_build_time_started,
            dist: self.job_dist,
            end_status: self.job_end_status.clone(),
            end_time_finished: self.job_end_time_finished,
            end_time_started: self.job_end_time_started,
            system: self.job_system.clone(),
            time_created: self.job_time_created,
        }
    }

    async fn mk_input(&self, build_status: &str) -> Result<Value, Error> {
        let evaluation = self.evaluation().await?;
        let jobset = evaluation.jobset().await?;
        let project = jobset.project().await?;
        Ok(json!({
            "drv": self.job_build_drv,
            "evaluation": evaluation.evaluation_num,
            "url": jobset.jobset_url,
            "url_locked": evaluation.evaluation_url_locked,
            "job": self.job_name,
            "jobset": jobset.jobset_name,
            "legacy": jobset.jobset_legacy,
            "out": self.job_build_out,
            "project": project.project_name,
            "status": build_status,
            "system": self.job_system,
        }))
    }

    pub async fn run(self) -> Result<(), Error> {
        use crate::time::now;
        let id = self.job_id;
        let drv = nix::DrvPath::new(&self.job_build_drv);

        // FIXME?
        let handle_1 = self.handle().await?;
        let handle_2 = handle_1.clone();
        let handle_3 = handle_1.clone();
        let handle_4 = handle_1.clone();
        let handle_5 = handle_1.clone();
        let job_1 = self;
        let job_2 = job_1.clone();

        // TODO: factor out common code between `begin` and `end`
        let task_begin = async move {
            let mut conn = connection().await;
            let _ = diesel::update(jobs.find(id))
                .set(job_begin_time_started.eq(now()))
                .execute(&mut *conn);
            drop(conn);

            let evaluation = job_1.evaluation().await?;
            let jobset = evaluation.jobset().await?;
            let project = jobset.project().await?;

            let input = job_1.mk_input(&"pending".to_string()).await?;
            let default_log = serde_json::to_string_pretty(&input).unwrap();
            let log = if let Some(path) = evaluation.evaluation_actions_path {
                if Path::new(&format!("{}/begin", path)).exists() {
                    let (_, log) = actions::run(
                        &project.project_key,
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
        let finish_begin = move |r| async move {
            use handles::Log::*;
            let status = match r {
                Some(Ok(log)) => {
                    let _ = Log::new(Begin(handle_1.clone()), log).await;
                    "success"
                }
                Some(Err(_)) => {
                    let _ = Log::new(Begin(handle_1.clone()), "TODO".to_string()).await;
                    "error"
                }
                None => "canceled",
            };
            let mut conn = connection().await;
            let _ = diesel::update(jobs.find(id))
                .set((
                    job_begin_status.eq(status),
                    job_begin_time_finished.eq(now()),
                ))
                .execute(&mut *conn);
            drop(conn);
            log_event(Event::JobUpdated(handle_2)).await;
        };
        JOBS_BEGIN.run(id, task_begin, finish_begin).await?;

        // FIXME: write a more intelligent build manager
        let (sender, receiver) = tokio::sync::oneshot::channel::<String>();
        let task_build = async move {
            let mut conn = connection().await;
            let _ = diesel::update(jobs.find(id))
                .set(job_build_time_started.eq(now()))
                .execute(&mut *conn);
            drop(conn);
            nix::build(&drv).await?;
            Ok::<(), Error>(())
        };
        let finish_build = move |r| async move {
            let status = match r {
                Some(Ok(())) => "success",
                Some(Err(_)) => "error", // TODO: log error
                None => "canceled",
            };
            sender.send(status.to_string()).unwrap_or_else(|_| panic!());
            let mut conn = connection().await;
            let _ = diesel::update(jobs.find(id))
                .set((
                    job_build_status.eq(status),
                    job_build_time_finished.eq(now()),
                ))
                .execute(&mut *conn);
            drop(conn);
            log_event(Event::JobUpdated(handle_3)).await;
        };
        JOBS_BUILD.run(id, task_build, finish_build).await?;

        let task_end = async move {
            // wait for `begin` to finish
            JOBS_BEGIN.wait(&id).await;
            // wait for the build to finish
            JOBS_BUILD.wait(&id).await;
            let build_status = receiver.await.unwrap_or_else(|_| panic!());

            let mut conn = connection().await;
            let _ = diesel::update(jobs.find(id))
                .set(job_end_time_started.eq(now()))
                .execute(&mut *conn);
            drop(conn);

            let evaluation = job_2.evaluation().await?;
            let jobset = evaluation.jobset().await?;
            let project = jobset.project().await?;

            let input = job_2.mk_input(&build_status).await?;
            let default_log = serde_json::to_string_pretty(&input).unwrap();
            let log = if let Some(path) = evaluation.evaluation_actions_path {
                if Path::new(&format!("{}/end", path)).exists() {
                    let (_, log) = actions::run(
                        &project.project_key,
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
        let finish_end = move |r| async move {
            use handles::Log::*;
            let status = match r {
                Some(Ok(log)) => {
                    // TODO: handle errors
                    let _ = Log::new(End(handle_4), log).await;
                    "success"
                }
                Some(Err(_)) => {
                    // TODO: handle errors
                    let _ = Log::new(End(handle_4), "TODO".to_string()).await;
                    "error"
                }
                None => "canceled",
            };
            let mut conn = connection().await;
            let _ = diesel::update(jobs.find(id))
                .set((job_end_status.eq(status), job_end_time_finished.eq(now())))
                .execute(&mut *conn);
            drop(conn);
            log_event(Event::JobUpdated(handle_5)).await;
        };
        JOBS_END.run(id, task_end, finish_end).await?;

        Ok(())
    }
}
