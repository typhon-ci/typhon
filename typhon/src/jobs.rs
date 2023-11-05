use crate::connection;
use crate::error::Error;
use crate::handles;
use crate::log_event;
use crate::models;
use crate::responses;
use crate::runs;
use crate::schema;

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
    pub async fn get(handle: &handles::Job) -> Result<Self, Error> {
        let mut conn = connection().await;
        let (job, (evaluation, project)) = schema::jobs::table
            .inner_join(schema::evaluations::table.inner_join(schema::projects::table))
            .filter(schema::projects::name.eq(&handle.evaluation.project.name))
            .filter(schema::evaluations::num.eq(handle.evaluation.num as i64))
            .filter(schema::jobs::system.eq(&handle.system))
            .filter(schema::jobs::name.eq(&handle.name))
            .first(&mut *conn)
            .optional()?
            .ok_or(Error::JobNotFound(handle.clone()))?;
        Ok(Self {
            job,
            evaluation,
            project,
        })
    }

    pub fn info(&self) -> responses::JobInfo {
        responses::JobInfo {
            dist: self.job.dist,
            drv: self.job.drv.clone(),
            out: self.job.out.clone(),
            system: self.job.system.clone(),
        }
    }

    pub async fn new_run(self) -> Result<runs::Run, Error> {
        let mut conn = connection().await;

        // create a new run in the database
        let run = conn.transaction::<models::Run, Error, _>(|conn| {
            let time_created = OffsetDateTime::now_utc().unix_timestamp();
            let max = schema::runs::table
                .filter(schema::runs::job_id.eq(self.job.id))
                .select(diesel::dsl::max(schema::runs::num))
                .first::<Option<i64>>(conn)?
                .unwrap_or(0);
            let num = max + 1;
            let new_run = models::NewRun {
                job_id: self.job.id,
                num,
                time_created,
            };
            let run = diesel::insert_into(schema::runs::table)
                .values(&new_run)
                .get_result::<models::Run>(conn)?;
            Ok(run)
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
        drop(conn);

        log_event(Event::RunNew(run.handle())).await;

        run.run().await?;

        Ok(run)
    }
}
