use crate::connection;
use crate::error::Error;
use crate::gcroots;
use crate::handles;
use crate::log_event;
use crate::models::*;
use crate::nix;
use crate::responses;
use crate::schema::evaluations::dsl::*;
use crate::schema::jobs::dsl::*;
use crate::schema::jobsets::dsl::*;
use crate::Event;
use crate::EVALUATIONS;

use diesel::prelude::*;

async fn evaluate_aux(id: i32, new_jobs: nix::NewJobs) -> Result<(), Error> {
    let mut conn = connection().await;
    let now = crate::time::now();
    let created_jobs = conn.transaction::<Vec<Job>, Error, _>(|conn| {
        new_jobs
            .iter()
            .map(|((system, name), (drv, dist))| {
                Ok(diesel::insert_into(jobs)
                    .values(&NewJob {
                        job_begin_status: "pending",
                        job_build_drv: &String::from(drv.path.clone()).as_str(),
                        job_build_out: drv
                            .outputs
                            .iter()
                            .last()
                            .expect("TODO: derivations can have multiple outputs")
                            .1,
                        job_build_status: "pending",
                        job_dist: *dist,
                        job_end_status: "pending",
                        job_evaluation: id,
                        job_name: name,
                        job_system: system,
                        job_time_created: now,
                    })
                    .get_result(conn)?)
            })
            .collect()
    })?;
    drop(conn);

    for job in created_jobs.into_iter() {
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
            num: self.evaluation_num,
        })
    }

    pub async fn info(&self) -> Result<responses::EvaluationInfo, Error> {
        use typhon_types::responses::JobSystemName;
        let mut conn = connection().await;
        let jobs_ = jobs
            .filter(job_evaluation.eq(self.evaluation_id))
            .load::<Job>(&mut *conn)?
            .iter()
            .map(|job| JobSystemName {
                system: job.job_system.clone(),
                name: job.job_name.clone(),
            })
            .collect();
        drop(conn);
        Ok(responses::EvaluationInfo {
            actions_path: self.evaluation_actions_path.clone(),
            jobs: jobs_,
            status: self.evaluation_status.clone(),
            time_created: self.evaluation_time_created,
            time_finished: self.evaluation_time_finished,
            url_locked: self.evaluation_url_locked.clone(),
        })
    }

    pub async fn jobset(&self) -> Result<Jobset, Error> {
        let mut conn = connection().await;
        Ok(jobsets
            .find(self.evaluation_jobset)
            .first::<Jobset>(&mut *conn)?)
    }

    pub async fn run(self) -> () {
        use handles::Log::*;

        // TODO: error management
        let handle = self.handle().await.unwrap();
        let jobset = self.jobset().await.unwrap();
        let id = self.evaluation_id;
        let task =
            async move { nix::eval_jobs(&self.evaluation_url_locked, jobset.jobset_legacy).await };
        let f = move |r: Option<Result<nix::NewJobs, nix::Error>>| async move {
            // TODO: when logging, hide internal error messages?
            let status = match r {
                Some(Ok(new_jobs)) => match evaluate_aux(id, new_jobs).await {
                    Ok(()) => "success",
                    Err(e) => {
                        // TODO: handle errors
                        let _ = Log::new(Eval(handle.clone()), e.to_string()).await;
                        "error"
                    }
                },
                Some(Err(e)) => {
                    // TODO: handle errors
                    let _ = Log::new(Eval(handle.clone()), e.to_string()).await;
                    "error"
                }
                None => "canceled",
            };

            // update the evaluation status
            let mut conn = connection().await;
            let _ = diesel::update(evaluations.find(id))
                .set(evaluation_status.eq(status))
                .execute(&mut *conn);
            gcroots::update(&mut *conn);
            drop(conn);

            log_event(Event::EvaluationFinished(handle));
        };
        EVALUATIONS.run(id, task, f).await;
    }
}
