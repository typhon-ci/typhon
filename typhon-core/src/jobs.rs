use crate::error::Error;
use crate::handles;
use crate::log_event;
use crate::models;
use crate::responses;
use crate::runs;
use crate::schema;
use crate::Conn;

use typhon_types::*;

use diesel::prelude::*;
use time::OffsetDateTime;

#[derive(Clone)]
pub struct Job {
    pub job: models::Job,
    pub evaluation: models::Evaluation,
    pub project: models::Project,
}

impl Job {
    pub fn get(conn: &mut Conn, handle: &handles::Job) -> Result<Self, Error> {
        let (job, (evaluation, project)) = schema::jobs::table
            .inner_join(schema::evaluations::table.inner_join(schema::projects::table))
            .filter(schema::evaluations::uuid.eq(handle.evaluation.uuid.to_string()))
            .filter(schema::jobs::name.eq(&handle.name))
            .first(conn)
            .optional()?
            .ok_or(Error::JobNotFound(handle.clone()))?;
        Ok(Self {
            job,
            evaluation,
            project,
        })
    }

    pub fn handle(&self) -> handles::Job {
        use std::str::FromStr;
        use uuid::Uuid;
        handles::Job {
            evaluation: handles::evaluation(Uuid::from_str(&self.evaluation.uuid).unwrap()),
            name: self.job.name.clone(),
        }
    }

    pub fn info(&self, conn: &mut Conn) -> Result<responses::JobInfo, Error> {
        let handle = self.handle();
        crate::evaluations::Evaluation::jobs(
            &handles::project(self.project.name.clone()),
            &handle.evaluation,
            self.evaluation.id,
            Some(handle.name.clone()),
            conn,
        )?
        .get(&handle.name)
        .cloned()
        .ok_or(Error::JobNotFound(handle))
    }

    /** Create a new run in the database, without running it. */
    pub fn new_run(&self, conn: &mut Conn) -> Result<runs::Run, Error> {
        let run = conn.transaction::<models::Run, Error, _>(|conn| {
            let num = self.job.tries + 1;
            diesel::update(&self.job)
                .set(schema::jobs::tries.eq(num))
                .execute(conn)?;
            let new_run = models::NewRun {
                job_id: self.job.id,
                num,
                time_created: OffsetDateTime::now_utc().unix_timestamp(),
            };
            Ok(diesel::insert_into(schema::runs::table)
                .values(&new_run)
                .get_result::<models::Run>(conn)?)
        })?;
        let run = runs::Run {
            begin: None,
            end: None,
            build: None,
            project: self.project.clone(),
            evaluation: self.evaluation.clone(),
            job: self.job.clone(),
            run,
        };
        log_event(Event::RunNew(run.handle()));
        Ok(run)
    }

    pub fn rerun(&self, conn: &mut Conn) -> Result<(), Error> {
        // TODO
        // We should only allow rerunning a job when no other run is pending for
        // that job. But we first need to rework runs, as it is currently hard
        // to know wether a run is finished or not.
        self.new_run(conn)?;
        Ok(())
    }
}
