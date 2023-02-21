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
    let mut conn = connection().await;
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
    drop(conn);

    for (build, job) in created_jobs.into_iter() {
        build.run().await;
        job.run().await;
    }

    Ok(())
}

impl Evaluation {
    pub async fn cancel(&self) -> Result<(), Error> {
        let r = EVALUATIONS.get().unwrap().cancel(self.evaluation_id).await;
        if r {
            Ok(())
        } else {
            Err(Error::EvaluationNotRunning(self.handle().await?))
        }
    }

    pub async fn get(evaluation_handle: &handles::Evaluation) -> Result<Self, Error> {
        let handles::pattern!(project_name_, jobset_name_, evaluation_num_) = evaluation_handle;
        let jobset = Jobset::get(&evaluation_handle.jobset).await?;
        let mut conn = connection().await;
        Ok(evaluations
            .filter(evaluation_jobset.eq(jobset.jobset_id))
            .filter(evaluation_num.eq(evaluation_num_))
            .first::<Evaluation>(&mut *conn)
            .map_err(|_| {
                Error::EvaluationNotFound(handles::evaluation((
                    project_name_.to_string(),
                    jobset_name_.to_string(),
                    *evaluation_num_,
                )))
            })?)
    }

    pub async fn handle(&self) -> Result<handles::Evaluation, Error> {
        Ok(handles::Evaluation {
            jobset: self.jobset().await?.handle().await?,
            evaluation: self.evaluation_num,
        })
    }

    pub async fn info(&self) -> Result<responses::EvaluationInfo, Error> {
        let mut conn = connection().await;
        let jobs_names = jobs
            .filter(job_evaluation.eq(self.evaluation_id))
            .load::<Job>(&mut *conn)?
            .iter()
            .map(|job| job.job_name.clone())
            .collect();
        drop(conn);
        Ok(responses::EvaluationInfo {
            actions_path: self.evaluation_actions_path.clone(),
            flake_locked: self.evaluation_flake_locked.clone(),
            jobs: jobs_names,
            status: self.evaluation_status.clone(),
            time_created: self.evaluation_time_created,
        })
    }

    pub async fn jobset(&self) -> Result<Jobset, Error> {
        let mut conn = connection().await;
        Ok(jobsets
            .find(self.evaluation_jobset)
            .first::<Jobset>(&mut *conn)?)
    }

    pub async fn run(self) -> () {
        let handle = self.handle().await.unwrap(); // TODO
        let id = self.evaluation_id;
        let task = async move {
            let expr = format!("{}#typhonJobs", self.evaluation_flake_locked);
            let attrset = nix::eval(expr).await?;
            let attrset = attrset.as_object().expect("unexpected Nix output"); // TODO: this is unsafe
            let mut jobs_: HashMap<String, String> = HashMap::new();
            for (name, _) in attrset.iter() {
                let expr = format!("{}#typhonJobs.{}", self.evaluation_flake_locked, name);
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
                        let _ = Log::new(handles::Log::Evaluation(handle.clone()), e.to_string());
                        "error"
                    }
                },
                Some(Err(e)) => {
                    let _ = Log::new(handles::Log::Evaluation(handle.clone()), e.to_string());
                    "error"
                }
                None => "canceled",
            };

            // update the evaluation status
            let mut conn = connection().await;
            let _ = diesel::update(evaluations.find(id))
                .set(evaluation_status.eq(status))
                .execute(&mut *conn);
            drop(conn);

            log_event(Event::EvaluationFinished(handle));
        };
        EVALUATIONS.get().unwrap().run(id, task, f).await;
    }
}
