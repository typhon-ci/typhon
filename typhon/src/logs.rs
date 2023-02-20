use crate::connection;
use crate::error::Error;
use crate::models::*;
use crate::schema::logs::dsl::*;
use diesel::prelude::*;
use typhon_types::*;

fn get_log_type(log: &handles::Log) -> &'static str {
    match log {
        handles::Log::Evaluation(_) => "evaluation",
        handles::Log::JobBegin(_) => "job_begin",
        handles::Log::JobEnd(_) => "job_end",
    }
}

impl Log {
    pub async fn new(log_handle: handles::Log, stderr: String) -> Result<Self, Error> {
        let ty = get_log_type(&log_handle);
        let mut new_log = NewLog {
            log_evaluation: None,
            log_job: None,
            log_stderr: &stderr,
            log_type: &ty,
        };
        match log_handle {
            handles::Log::Evaluation(h) => {
                let evaluation = Evaluation::get(&h).await?;
                new_log.log_evaluation = Some(evaluation.evaluation_id);
            }
            handles::Log::JobBegin(h) | handles::Log::JobEnd(h) => {
                let job = Job::get(&h).await?;
                new_log.log_job = Some(job.job_id);
            }
        };
        let mut conn = connection().await;
        Ok(diesel::insert_into(logs)
            .values(&new_log)
            .get_result(&mut *conn)?)
    }

    pub async fn get(log_handle: handles::Log) -> Result<Self, Error> {
        let ty = get_log_type(&log_handle).to_string();
        let req = logs.filter(log_type.eq(ty));
        (match &log_handle {
            handles::Log::Evaluation(h) => {
                let evaluation = Evaluation::get(&h).await?;
                let mut conn = connection().await;
                req.filter(log_evaluation.eq(Some(evaluation.evaluation_id)))
                    .first::<Log>(&mut *conn)
            }
            handles::Log::JobBegin(h) | handles::Log::JobEnd(h) => {
                let job = Job::get(&h).await?;
                let mut conn = connection().await;
                req.filter(log_job.eq(Some(job.job_id)))
                    .first::<Log>(&mut *conn)
            }
        })
        .map_err(|_| Error::LogNotFound(log_handle))
    }
}
