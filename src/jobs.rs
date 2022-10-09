use crate::error::Error;
use crate::models::*;
use crate::schema::jobs::dsl::*;
use crate::{connection, JOBS};
use diesel::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct JobHandle {
    pub project: String,
    pub jobset: String,
    pub evaluation: i32,
    pub job: String,
}

impl std::fmt::Display for JobHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}:{}",
            self.project, self.jobset, self.evaluation, self.job
        )
    }
}

#[derive(Debug, Serialize)]
pub struct JobInfo {
    pub project: String,
    pub jobset: String,
    pub evaluation: i64,
    pub build: String,
    pub status: String,
}

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
        todo!()
    }

    pub fn info(&self) -> Result<JobInfo, Error> {
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
            let conn = &mut connection();
            let _ = diesel::update(jobs.find(id))
                .set(job_status.eq(status))
                .execute(conn);
        };
        JOBS.get().unwrap().run(id, task, f);
    }
}
