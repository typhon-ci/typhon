use crate::error::Error;
use crate::evaluations;
use crate::gcroots;
use crate::models;
use crate::nix;
use crate::schema;
use crate::Conn;
use crate::DbPool;
use crate::{handles, responses};
use crate::{log_event, Event};

use diesel::prelude::*;
use serde::Deserialize;
use time::OffsetDateTime;

#[derive(Clone)]
pub struct Jobset {
    pub jobset: models::Jobset,
    pub project: models::Project,
}

#[derive(Deserialize, PartialEq)]
pub struct JobsetDecl {
    pub flake: bool,
    pub url: String,
}

impl Jobset {
    pub fn delete(&self, conn: &mut Conn) -> Result<(), Error> {
        diesel::delete(schema::jobsets::table.find(&self.jobset.id)).execute(conn)?;
        Ok(())
    }

    pub fn evaluate(&self, conn: &mut Conn, force: bool) -> Result<handles::Evaluation, Error> {
        use crate::tasks;

        let url = nix::lock(&self.jobset.url)?;

        let preexisting = schema::evaluations::table
            .inner_join(schema::tasks::table)
            .filter(schema::evaluations::jobset_name.eq(&self.jobset.name))
            .filter(schema::evaluations::url.eq(&url))
            .first::<(models::Evaluation, models::Task)>(conn)
            .optional()?;

        let evaluation = match (preexisting, force) {
            (Some((evaluation, task)), false) => evaluations::Evaluation {
                project: self.project.clone(),
                evaluation,
                task: tasks::Task { task },
            },
            _ => self.new_evaluation(conn, &url)?,
        };

        Ok(evaluation.handle())
    }

    pub fn decl(&self) -> JobsetDecl {
        JobsetDecl {
            flake: self.jobset.flake,
            url: self.jobset.url.clone(),
        }
    }

    pub fn get(conn: &mut Conn, handle: &handles::Jobset) -> Result<Self, Error> {
        let (jobset, project) = schema::jobsets::table
            .inner_join(schema::projects::table)
            .filter(schema::projects::name.eq(&handle.project.name))
            .filter(schema::jobsets::name.eq(&handle.name))
            .first(conn)
            .optional()?
            .ok_or(Error::JobsetNotFound(handle.clone()))?;
        Ok(Jobset { jobset, project })
    }

    pub fn info(&self) -> responses::JobsetInfo {
        responses::JobsetInfo {
            flake: self.jobset.flake,
            url: self.jobset.url.clone(),
        }
    }

    fn new_evaluation(
        &self,
        conn: &mut Conn,
        url: &String,
    ) -> Result<evaluations::Evaluation, Error> {
        use crate::tasks;

        let (evaluation, task) =
            conn.transaction::<(models::Evaluation, tasks::Task), Error, _>(|conn| {
                let task = tasks::Task::new(conn)?;
                let time_created = OffsetDateTime::now_utc().unix_timestamp();
                let max = schema::evaluations::table
                    .filter(schema::evaluations::project_id.eq(self.project.id))
                    .select(diesel::dsl::max(schema::evaluations::num))
                    .first::<Option<i64>>(conn)?
                    .unwrap_or(0);
                let num = max + 1;
                let new_evaluation = models::NewEvaluation {
                    actions_path: self.project.actions_path.as_ref().map(|s| s.as_str()),
                    flake: self.jobset.flake,
                    jobset_name: &self.jobset.name,
                    num,
                    project_id: self.project.id,
                    task_id: task.task.id,
                    time_created,
                    url: &url,
                };
                let evaluation = diesel::insert_into(schema::evaluations::table)
                    .values(&new_evaluation)
                    .get_result::<models::Evaluation>(conn)?;

                Ok((evaluation, task))
            })?;
        let evaluation = evaluations::Evaluation {
            project: self.project.clone(),
            evaluation,
            task,
        };

        let run = {
            let evaluation = evaluation.clone();
            move |sender| evaluation.run(sender)
        };

        let finish = {
            let evaluation = evaluation.clone();
            move |r, pool: &DbPool| evaluation.finish(r, pool)
        };

        evaluation.task.run(conn, run, finish)?;

        log_event(Event::EvaluationNew(evaluation.handle()));

        gcroots::update(conn);

        Ok(evaluation)
    }
}
