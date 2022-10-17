use crate::error::Error;
use crate::models::*;
use crate::nix;
use crate::schema::builds::dsl::*;
use crate::schema::evaluations::dsl::*;
use crate::schema::jobs::dsl::*;
use crate::schema::jobsets::dsl::*;
use crate::{connection, EVALUATIONS};
use crate::{handles, responses};
use crate::{log_event, Event};
use diesel::prelude::*;
use std::collections::HashMap;
use substring::Substring;

fn evaluate_aux(id: i32, new_jobs: HashMap<String, String>) -> Result<(), Error> {
    let conn = &mut *connection();
    conn.transaction::<(), Error, _>(|conn| {
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
                    log_event(Event::BuildNew(build.handle()?));
                    build.clone().run();
                    Ok(build)
                }
            }?;

            // Create job
            let new_job = NewJob {
                job_build: build.build_id,
                job_evaluation: id,
                job_name: &name,
                job_status: "pending",
            };
            let job: Job = diesel::insert_into(jobs)
                .values(&new_job)
                .get_result(conn)?;
            job.run();
        }
        Ok(())
    })?;
    Ok(())
}

impl Evaluation {
    pub fn cancel(&self) -> Result<(), Error> {
        todo!()
    }

    pub fn get(evaluation_handle: &handles::Evaluation) -> Result<Self, Error> {
        let handles::pattern!(project_name_, jobset_name_, evaluation_num_) = evaluation_handle;
        let jobset = Jobset::get(&evaluation_handle.jobset)?;
        let conn = &mut *connection();
        Ok(evaluations
            .filter(evaluation_jobset.eq(jobset.jobset_id))
            .filter(evaluation_num.eq(evaluation_num_))
            .first::<Evaluation>(conn)
            .map_err(|_| {
                Error::EvaluationNotFound(handles::evaluation(
                    project_name_.to_string(),
                    jobset_name_.to_string(),
                    *evaluation_num_,
                ))
            })?)
    }

    pub fn handle(&self) -> Result<handles::Evaluation, Error> {
        Ok(handles::Evaluation {
            jobset: self.jobset()?.handle()?,
            evaluation: self.evaluation_num,
        })
    }

    pub fn info(&self) -> Result<responses::EvaluationInfo, Error> {
        let jobset = self.jobset()?;
        let project = jobset.project()?;
        Ok(responses::EvaluationInfo {
            project: project.project_name.clone(),
            jobset: jobset.jobset_name.clone(),
            locked_flake: self.evaluation_locked_flake.clone(),
            time_created: self.evaluation_time_created,
            status: self.evaluation_status.clone(),
            jobs: HashMap::new(), //TODO
        })
    }

    pub fn jobset(&self) -> Result<Jobset, Error> {
        let conn = &mut *connection();
        Ok(jobsets.find(self.evaluation_jobset).first::<Jobset>(conn)?)
    }

    pub fn run(self) -> () {
        let handle = self.handle().unwrap(); // TODO
        let id = self.evaluation_id;
        let task = async move {
            let expr = format!("{}#typhonJobs", self.evaluation_locked_flake);
            let attrset = nix::eval(expr)?;
            let attrset = attrset.as_object().expect("unexpected Nix output"); // TODO: this is unsafe
            let mut jobs_: HashMap<String, String> = HashMap::new();
            for (name, _) in attrset.iter() {
                let expr = format!("{}#typhonJobs.{}", self.evaluation_locked_flake, name);
                let drv_path = nix::derivation_path(expr)?;
                jobs_.insert(name.to_string(), drv_path);
            }
            Ok::<_, Error>(jobs_)
        };
        let f = move |r| {
            let status = match r {
                Some(Ok(new_jobs)) => match evaluate_aux(id, new_jobs) {
                    Ok(()) => "success",
                    Err(_) => {
                        // TODO: log error to the user
                        "error"
                    }
                },
                Some(Err(_)) => "error", // TODO: log error to the user
                None => "canceled",
            };
            let conn = &mut *connection();
            let _ = diesel::update(evaluations.find(id))
                .set(evaluation_status.eq(status))
                .execute(conn);
            log_event(Event::EvaluationFinished(handle));
        };
        EVALUATIONS.get().unwrap().run(id, task, f);
    }
}
