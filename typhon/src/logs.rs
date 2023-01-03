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
    pub fn new(
        conn: &mut SqliteConnection,
        log: handles::Log,
        stderr: String,
    ) -> Result<Self, Error> {
        let ty = get_log_type(&log);
        let mut new_log = NewLog {
            log_evaluation: None,
            log_job: None,
            log_stderr: &stderr,
            log_type: &ty,
        };
        match log {
            handles::Log::Evaluation(h) => {
                let evaluation = Evaluation::get(conn, &h)?;
                new_log.log_evaluation = Some(evaluation.evaluation_id);
            }
            handles::Log::JobBegin(h) | handles::Log::JobEnd(h) => {
                let job = Job::get(conn, &h)?;
                new_log.log_job = Some(job.job_id);
            }
        };
        Ok(diesel::insert_into(logs)
            .values(&new_log)
            .get_result(conn)?)
    }

    pub fn get(conn: &mut SqliteConnection, log: handles::Log) -> Result<Self, Error> {
        let ty = get_log_type(&log).to_string();
        let req = logs.filter(log_type.eq(ty));
        (match &log {
            handles::Log::Evaluation(h) => {
                let evaluation = Evaluation::get(conn, &h)?;
                req.filter(log_evaluation.eq(Some(evaluation.evaluation_id)))
                    .first::<Log>(conn)
            }
            handles::Log::JobBegin(h) | handles::Log::JobEnd(h) => {
                let job = Job::get(conn, &h)?;
                req.filter(log_job.eq(Some(job.job_id))).first::<Log>(conn)
            }
        })
        .map_err(|_| Error::LogNotFound(log))
    }
}
