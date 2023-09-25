use crate::connection;
use crate::error::Error;
use crate::models::*;
use crate::nix;
use crate::schema::builds::dsl::*;
use crate::schema::evaluations::dsl::*;
use crate::schema::jobs::dsl::*;
use crate::schema::jobsets::dsl::*;
use crate::CURRENT_SYSTEM;
use crate::EVALUATIONS;
use crate::{handles, responses};
use crate::{log_event, Event};
use diesel::prelude::*;
use std::collections::HashMap;

type JobName = String;
type JobDrvMap = HashMap<JobName, (nix::Derivation, bool)>;

async fn evaluate_aux(id: i32, new_jobs: JobDrvMap) -> Result<(), Error> {
    let mut conn = connection().await;
    let created_jobs = conn.transaction::<Vec<(Build, Job)>, Error, _>(|conn| {
        new_jobs
            .iter()
            .map(|(name, (drv, dist))| {
                let hash = &drv.path.hash();
                let build = builds
                    .filter(build_hash.eq(hash))
                    .load::<Build>(conn)?
                    .last()
                    .cloned()
                    .map(Ok::<_, Error>)
                    .unwrap_or_else(|| {
                        let build: Build = diesel::insert_into(builds)
                            .values(&NewBuild {
                                build_drv: &String::from(drv.path.clone()).as_str(),
                                build_hash: hash,
                                build_out: drv
                                    .outputs
                                    .iter()
                                    .last()
                                    .expect("TODO: derivations can have multiple outputs")
                                    .1,
                                build_status: "pending",
                            })
                            .get_result(conn)?;
                        log_event(Event::BuildNew(build.handle()));
                        Ok(build)
                    })?;

                // Create job
                let job: Job = diesel::insert_into(jobs)
                    .values(&NewJob {
                        job_build: build.build_id,
                        job_dist: *dist,
                        job_evaluation: id,
                        job_name: &name,
                        job_status: "begin",
                        job_system: &*CURRENT_SYSTEM,
                    })
                    .get_result(conn)?;
                Ok((build, job))
            })
            .collect()
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
        let r = EVALUATIONS.cancel(self.evaluation_id).await;
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
            let expr = format!(
                "{}#typhonJobs.{}",
                self.evaluation_flake_locked, *CURRENT_SYSTEM,
            );
            let mut jobs_ = JobDrvMap::new();
            for job in nix::eval(expr.clone())
                .await?
                .as_object()
                .expect("unexpected Nix output")
                .keys()
                .cloned()
            {
                jobs_.insert(
                    job.clone(),
                    (
                        nix::derivation(&format!("{expr}.{job}")).await?,
                        nix::eval(format!("{expr}.{job}.passthru.typhonDist"))
                            .await
                            .map(|json| json.as_bool().unwrap_or(false))
                            .unwrap_or(false),
                    ),
                );
            }
            Ok(jobs_)
        };
        let f = move |r: Option<Result<JobDrvMap, Error>>| async move {
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
        EVALUATIONS.run(id, task, f).await;
    }
}
