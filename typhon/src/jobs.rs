use crate::actions;
use crate::connection;
use crate::error::Error;
use crate::handles;
use crate::models::*;
use crate::responses;
use crate::schema::builds::dsl::*;
use crate::schema::evaluations::dsl::*;
use crate::schema::jobs::dsl::*;
use crate::{log_event, Event};
use crate::{BUILDS, JOBS, SETTINGS};
use diesel::prelude::*;
use serde_json::{json, Value};
use std::path::Path;

impl Job {
    pub async fn build(&self) -> Result<Build, Error> {
        let mut conn = connection().await;
        Ok(builds.find(self.job_build).first::<Build>(&mut *conn)?)
    }

    pub async fn cancel(&self) -> Result<(), Error> {
        let r = JOBS.get().unwrap().cancel(self.job_id).await;
        if r {
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
        let handles::pattern!(project_name_, jobset_name_, evaluation_num_, job_name_) = job_handle;
        let evaluation = Evaluation::get(&job_handle.evaluation).await?;
        let mut conn = connection().await;
        Ok(jobs
            .filter(job_evaluation.eq(evaluation.evaluation_id))
            .filter(job_name.eq(job_name_))
            .first::<Job>(&mut *conn)
            .map_err(|_| {
                Error::JobNotFound(handles::job((
                    project_name_.clone(),
                    jobset_name_.clone(),
                    *evaluation_num_,
                    job_name_.clone(),
                )))
            })?)
    }

    pub async fn handle(&self) -> Result<handles::Job, Error> {
        Ok(handles::Job {
            evaluation: self.evaluation().await?.handle().await?,
            job: self.job_name.clone(),
        })
    }

    pub async fn info(&self) -> Result<responses::JobInfo, Error> {
        let mut conn = connection().await;
        let build = builds.find(self.job_build).first::<Build>(&mut *conn)?;
        Ok(responses::JobInfo {
            build_handle: handles::build(build.build_hash.clone()),
            build_infos: build.into(),
            status: self.job_status.clone(),
        })
    }

    async fn mk_input(&self) -> Result<Value, Error> {
        let evaluation = self.evaluation().await?;
        let jobset = evaluation.jobset().await?;
        let project = jobset.project().await?;
        let build = self.build().await?;
        Ok(json!({
            "build": build.build_hash,
            "data": SETTINGS.get().unwrap().json,
            "evaluation": evaluation.evaluation_num,
            "flake": jobset.jobset_flake,
            "flake_locked": evaluation.evaluation_flake_locked,
            "job": self.job_name,
            "jobset": jobset.jobset_name,
            "out": build.build_out,
            "project": project.project_name,
            "status": build.build_status,
        }))
    }

    pub async fn run(self) -> () {
        let id = self.job_id;

        let handle = self.handle().await.unwrap(); // TODO
        let handle_bis = handle.clone();

        let task = async move {
            // abort if actions are not defined
            let evaluation = self.evaluation().await?;
            let path = match &evaluation.evaluation_actions_path {
                None => return Ok(()),
                Some(path) => path,
            };

            let jobset = evaluation.jobset().await?;
            let project = jobset.project().await?;

            {
                // run action `begin`
                let mut conn = connection().await;
                let _ = diesel::update(jobs.find(id))
                    .set(job_status.eq("begin"))
                    .execute(&mut *conn);
                drop(conn);

                log_event(Event::JobUpdated(handle_bis.clone()));

                let input = self.mk_input().await?;

                let log = if Path::new(&format!("{}/begin", path)).exists() {
                    let (_, log) = actions::run(
                        &project.project_key,
                        &format!("{}/begin", path),
                        &format!("{}/secrets", path),
                        &input,
                    )
                    .await?;
                    log
                } else {
                    serde_json::to_string_pretty(&input).unwrap() // TODO
                };

                // save the log
                let _ = Log::new(handles::Log::JobBegin(handle_bis.clone()), log).await?;
            }

            // wait for build
            let mut conn = connection().await;
            let _ = diesel::update(jobs.find(id))
                .set(job_status.eq("waiting"))
                .execute(&mut *conn);
            drop(conn);

            log_event(Event::JobUpdated(handle_bis.clone()));

            BUILDS.get().unwrap().wait(&self.job_build).await;

            {
                // run action `end`
                let mut conn = connection().await;
                let _ = diesel::update(jobs.find(id))
                    .set(job_status.eq("end"))
                    .execute(&mut *conn);
                drop(conn);

                log_event(Event::JobUpdated(handle_bis.clone()));

                let input = self.mk_input().await?;

                let log = if Path::new(&format!("{}/end", path)).exists() {
                    let (_, log) = actions::run(
                        &project.project_key,
                        &format!("{}/end", path),
                        &format!("{}/secrets", path),
                        &input,
                    )
                    .await?;
                    log
                } else {
                    serde_json::to_string_pretty(&input).unwrap() // TODO
                };

                // save the log
                let _ = Log::new(handles::Log::JobEnd(handle_bis), log).await?;
            }

            Ok::<(), Error>(())
        };
        let f = move |r| async move {
            let status = match r {
                Some(Ok(())) => "success",
                Some(Err(_)) => "error", // TODO: log error
                None => "canceled",
            };
            let mut conn = connection().await;
            let _ = diesel::update(jobs.find(id))
                .set(job_status.eq(status))
                .execute(&mut *conn);
            drop(conn);
            log_event(Event::JobUpdated(handle));
        };
        JOBS.get().unwrap().run(id, task, f).await;
    }
}
