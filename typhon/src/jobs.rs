use crate::error::Error;
use crate::models::*;
use crate::schema::jobs::dsl::*;
use crate::{connection, JOBS};
use diesel::prelude::*;
use serde::Serialize;
use crate::{responses};

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
            let conn = &mut connection();
            let _ = diesel::update(jobs.find(id))
                .set(job_status.eq(status))
                .execute(conn);
        };
        JOBS.get().unwrap().run(id, task, f);
    }
}
