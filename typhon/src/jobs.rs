use crate::connection;
use crate::error::Error;
use crate::handles;
use crate::models::*;
use crate::responses;
use crate::schema::builds::dsl::*;
use crate::schema::jobs::dsl::*;
use crate::JOBS;
use diesel::prelude::*;

impl Job {
    pub fn cancel(&self) -> Result<(), Error> {
        todo!()
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

    pub fn info(&self, conn: &mut SqliteConnection) -> Result<responses::JobInfo, Error> {
        let build = builds.find(self.job_build).first::<Build>(conn)?;
        Ok(responses::JobInfo {
            build: handles::build(build.build_hash.clone()),
            status: self.job_status.clone(),
        })
    }

    pub async fn run(self) -> () {
        let id = self.job_id;
        let task = async move { Err(Error::Todo) };
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
