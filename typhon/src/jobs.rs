use crate::actions;
use crate::connection;
use crate::error::Error;
use crate::handles;
use crate::models::*;
use crate::responses;
use crate::schema::builds::dsl::*;
use crate::schema::evaluations::dsl::*;
use crate::schema::jobs::dsl::*;
use crate::{BUILDS, JOBS, SETTINGS};
use diesel::prelude::*;
use serde_json::json;
use std::path::Path;

impl Job {
    pub fn build(&self, conn: &mut SqliteConnection) -> Result<Build, Error> {
        Ok(builds.find(self.job_build).first::<Build>(conn)?)
    }

    pub async fn cancel(&self, conn: &mut SqliteConnection) -> Result<(), Error> {
        let r = JOBS.get().unwrap().cancel(self.job_id).await;
        if r {
            Ok(())
        } else {
            Err(Error::JobNotRunning(self.handle(conn)?))
        }
    }

    pub fn evaluation(&self, conn: &mut SqliteConnection) -> Result<Evaluation, Error> {
        Ok(evaluations
            .find(self.job_evaluation)
            .first::<Evaluation>(conn)?)
    }

    pub fn get(conn: &mut SqliteConnection, job_handle: &handles::Job) -> Result<Self, Error> {
        let handles::pattern!(project_name_, jobset_name_, evaluation_num_, job_name_) = job_handle;
        let evaluation = Evaluation::get(conn, &job_handle.evaluation)?;
        Ok(jobs
            .filter(job_evaluation.eq(evaluation.evaluation_id))
            .filter(job_name.eq(job_name_))
            .first::<Job>(conn)
            .map_err(|_| {
                Error::JobNotFound(handles::job((
                    project_name_.clone(),
                    jobset_name_.clone(),
                    *evaluation_num_,
                    job_name_.clone(),
                )))
            })?)
    }

    pub fn handle(&self, conn: &mut SqliteConnection) -> Result<handles::Job, Error> {
        Ok(handles::Job {
            evaluation: self.evaluation(conn)?.handle(conn)?,
            job: self.job_name.clone(),
        })
    }

    pub fn info(&self, conn: &mut SqliteConnection) -> Result<responses::JobInfo, Error> {
        let build = builds.find(self.job_build).first::<Build>(conn)?;
        Ok(responses::JobInfo {
            build: handles::build(build.build_hash.clone()),
            status: self.job_status.clone(),
        })
    }

    pub async fn run(self) -> () {
        let id = self.job_id;
        let task = async move {
            let mut conn = connection().await;

            // abort if actions are not defined
            let evaluation = self.evaluation(&mut conn)?;
            let path = match &evaluation.evaluation_actions_path {
                None => return Ok(()),
                Some(path) => path,
            };

            let jobset = evaluation.jobset(&mut conn)?;
            let project = jobset.project(&mut conn)?;
            let build = self.build(&mut conn)?;

            drop(conn);

            // run action `begin`
            if Path::new(&format!("{}/begin", path)).exists() {
                let input = json!({
                    "project": project.project_name,
                    "jobset": jobset.jobset_name,
                    "evaluation": evaluation.evaluation_num,
                    "job": self.job_name,
                    "build": build.build_hash,
                    "flake": jobset.jobset_flake,
                    "locked_flake": evaluation.evaluation_locked_flake,
                    "data": SETTINGS.get().unwrap().json,
                });
                let _ = actions::run(
                    &project.project_key,
                    &format!("{}/begin", path),
                    &format!("{}/secrets", path),
                    &input,
                )
                .await?;
            }

            // wait for build
            BUILDS.get().unwrap().wait(&self.job_build).await;
            let mut conn = connection().await;
            let build = self.build(&mut *conn)?;
            drop(conn);

            // run action `end`
            if Path::new(&format!("{}/end", path)).exists() {
                let input = json!({
                    "project": project.project_name,
                    "jobset": jobset.jobset_name,
                    "evaluation": evaluation.evaluation_num,
                    "job": self.job_name,
                    "build": build.build_hash,
                    "flake": jobset.jobset_flake,
                    "locked_flake": evaluation.evaluation_locked_flake,
                    "data": SETTINGS.get().unwrap().json,
                    "status": build.build_status,
                });
                let _ = actions::run(
                    &project.project_key,
                    &format!("{}/end", path),
                    &format!("{}/secrets", path),
                    &input,
                )
                .await?;
            }

            Ok::<(), Error>(())
        };
        let f = move |r| async move {
            let status = match r {
                Some(Ok(())) => "success",
                Some(Err(_)) => "error", // TODO: log error
                None => "canceled",
            };
            let conn = &mut *connection().await;
            let _ = diesel::update(jobs.find(id))
                .set(job_status.eq(status))
                .execute(conn);
        };
        JOBS.get().unwrap().run(id, task, f).await;
    }
}
