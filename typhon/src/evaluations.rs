use crate::connection;
use crate::error::Error;
use crate::models::*;
use crate::nix;
use crate::schema::builds::dsl::*;
use crate::schema::evaluations::dsl::*;
use crate::schema::jobs::dsl::*;
use crate::schema::jobsets::dsl::*;
use crate::EVALUATIONS;
use crate::{handles, responses};
use crate::{log_event, Event};
use diesel::prelude::*;
use std::collections::HashMap;
use substring::Substring;

async fn evaluate_aux(id: i32, new_jobs: HashMap<String, String>) -> Result<(), Error> {
    let conn = &mut *connection().await;
    let created_jobs = conn.transaction::<Vec<(Build, Job)>, Error, _>(|conn| {
        let mut created_jobs = vec![];
        for (name, drv) in new_jobs.iter() {
            let hash = drv.substring(11, 43).to_string();

            // Create and run build if it doesn't exist
            let build: Build = match builds
                .filter(build_hash.eq(&hash))
                .load::<Build>(conn)?
                .last()
            {
                Some(build) => Ok::<Build, Error>(build.clone()),
                None => {
                    let new_build = NewBuild {
                        build_hash: &hash,
                        build_drv: drv,
                        build_status: "pending",
                    };
                    let build: Build = diesel::insert_into(builds)
                        .values(&new_build)
                        .get_result(conn)?;
                    log_event(Event::BuildNew(build.handle()));
                    Ok(build)
                }
            }?;

            // Create job
            let new_job = NewJob {
                job_build: build.build_id,
                job_evaluation: id,
                job_name: &name,
                job_status: "begin",
            };
            let job: Job = diesel::insert_into(jobs)
                .values(&new_job)
                .get_result(conn)?;

            created_jobs.push((build, job));
        }
        Ok(created_jobs)
    })?;

    for (build, job) in created_jobs.into_iter() {
        build.run().await;
        job.run().await;
    }

    Ok(())
}

impl Evaluation {
    pub async fn cancel(&self, conn: &mut SqliteConnection) -> Result<(), Error> {
        let r = EVALUATIONS.get().unwrap().cancel(self.evaluation_id).await;
        if r {
            Ok(())
        } else {
            Err(Error::EvaluationNotRunning(self.handle(conn)?))
        }
    }

    pub fn get(
        conn: &mut SqliteConnection,
        evaluation_handle: &handles::Evaluation,
    ) -> Result<Self, Error> {
        let handles::pattern!(project_name_, jobset_name_, evaluation_num_) = evaluation_handle;
        let jobset = Jobset::get(conn, &evaluation_handle.jobset)?;
        Ok(evaluations
            .filter(evaluation_jobset.eq(jobset.jobset_id))
            .filter(evaluation_num.eq(evaluation_num_))
            .first::<Evaluation>(conn)
            .map_err(|_| {
                Error::EvaluationNotFound(handles::evaluation((
                    project_name_.to_string(),
                    jobset_name_.to_string(),
                    *evaluation_num_,
                )))
            })?)
    }

    pub fn handle(&self, conn: &mut SqliteConnection) -> Result<handles::Evaluation, Error> {
        Ok(handles::Evaluation {
            jobset: self.jobset(conn)?.handle(conn)?,
            evaluation: self.evaluation_num,
        })
    }

    pub fn info(&self, conn: &mut SqliteConnection) -> Result<responses::EvaluationInfo, Error> {
        let jobs_names = jobs
            .filter(job_evaluation.eq(self.evaluation_id))
            .load::<Job>(conn)?
            .iter()
            .map(|job| job.job_name.clone())
            .collect();
        Ok(responses::EvaluationInfo {
            actions_path: self.evaluation_actions_path.clone(),
            jobs: jobs_names,
            locked_flake: self.evaluation_locked_flake.clone(),
            status: self.evaluation_status.clone(),
            time_created: self.evaluation_time_created,
        })
    }

    pub fn jobset(&self, conn: &mut SqliteConnection) -> Result<Jobset, Error> {
        Ok(jobsets.find(self.evaluation_jobset).first::<Jobset>(conn)?)
    }

    pub async fn run(self, conn: &mut SqliteConnection) -> () {
        let handle = self.handle(conn).unwrap(); // TODO
        let id = self.evaluation_id;
        let task = async move {
            let expr = format!("{}#typhonJobs", self.evaluation_locked_flake);
            let attrset = nix::eval(expr).await?;
            let attrset = attrset.as_object().expect("unexpected Nix output"); // TODO: this is unsafe
            let mut jobs_: HashMap<String, String> = HashMap::new();
            for (name, _) in attrset.iter() {
                let expr = format!("{}#typhonJobs.{}", self.evaluation_locked_flake, name);
                let drv_path = nix::derivation_path(expr).await?;
                jobs_.insert(name.to_string(), drv_path);
            }
            Ok::<_, Error>(jobs_)
        };
        let f = move |r: Option<Result<_, Error>>| async move {
            // TODO: when logging, hide internal error messages?
            let status = match r {
                Some(Ok(new_jobs)) => match evaluate_aux(id, new_jobs).await {
                    Ok(()) => "success",
                    Err(e) => {
                        let mut conn = connection().await;
                        let _ = Log::new(
                            &mut *conn,
                            handles::Log::Evaluation(handle.clone()),
                            e.to_string(),
                        );
                        drop(conn);
                        "error"
                    }
                },
                Some(Err(e)) => {
                    let mut conn = connection().await;
                    let _ = Log::new(
                        &mut *conn,
                        handles::Log::Evaluation(handle.clone()),
                        e.to_string(),
                    );
                    drop(conn);
                    "error"
                }
                None => "canceled",
            };

            let mut conn = connection().await;

            // update the evaluation status
            let _ = diesel::update(evaluations.find(id))
                .set(evaluation_status.eq(status))
                .execute(&mut *conn);

            log_event(Event::EvaluationFinished(handle));
        };
        EVALUATIONS.get().unwrap().run(id, task, f).await;
    }
}
