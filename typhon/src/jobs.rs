use crate::error::Error;
use crate::handles;
use crate::models::*;
use crate::responses;
use crate::schema::jobs::dsl::*;
use crate::{connection, JOBS};
use diesel::prelude::*;

impl Job {
    pub fn cancel(&self) -> Result<(), Error> {
        todo!()
    }

    pub fn get(
        project_name_: &String,
        jobset_name_: &String,
        evaluation_num_: i32,
        job_name_: &String,
    ) -> Result<Self, Error> {
        let evaluation = Evaluation::get(project_name_, jobset_name_, evaluation_num_)?;
        let conn = &mut *connection();
        Ok(jobs
            .filter(job_evaluation.eq(evaluation.evaluation_id))
            .filter(job_name.eq(job_name_))
            .first::<Job>(conn)
            .map_err(|_| {
                Error::JobNotFound(handles::job(
                    project_name_.clone(),
                    jobset_name_.clone(),
                    evaluation_num_,
                    job_name_.clone(),
                ))
            })?)
    }

    pub fn info(&self) -> Result<responses::JobInfo, Error> {
        todo!()
    }

    pub fn run(self) -> () {
        let id = self.job_id;
        let task = async move { Err(Error::Todo) };
        let f = move |r| {
            let status = match r {
                Some(Ok(())) => "success",
                Some(Err(_)) => "error", // TODO: log error
                None => "canceled",
            };
            let conn = &mut *connection();
            let _ = diesel::update(jobs.find(id))
                .set(job_status.eq(status))
                .execute(conn);
        };
        JOBS.get().unwrap().run(id, task, f);
    }
}
