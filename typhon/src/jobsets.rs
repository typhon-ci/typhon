use crate::connection;
use crate::error::Error;
use crate::evaluations;
use crate::gcroots;
use crate::models;
use crate::nix;
use crate::schema;
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
    pub async fn delete(&self) -> Result<(), Error> {
        let mut conn = connection().await;
        diesel::delete(schema::jobsets::table.find(&self.jobset.id)).execute(&mut *conn)?;
        Ok(())
    }

    pub async fn evaluate(&self, force: bool) -> Result<handles::Evaluation, Error> {
        use crate::tasks;

        let url = nix::lock(&self.jobset.url).await?;

        let mut conn = connection().await;
        let preexisting = schema::evaluations::table
            .inner_join(schema::tasks::table)
            .filter(schema::evaluations::jobset_name.eq(&self.jobset.name))
            .filter(schema::evaluations::url.eq(&url))
            .first::<(models::Evaluation, models::Task)>(&mut *conn)
            .optional()?;
        drop(conn);

        let evaluation = match (preexisting, force) {
            (Some((evaluation, task)), false) => evaluations::Evaluation {
                project: self.project.clone(),
                evaluation,
                task: tasks::Task { task },
            },
            _ => self.new_evaluation(&url).await?,
        };

        Ok(evaluation.handle())
    }

    pub fn decl(&self) -> JobsetDecl {
        JobsetDecl {
            flake: self.jobset.flake,
            url: self.jobset.url.clone(),
        }
    }

    pub async fn get(handle: &handles::Jobset) -> Result<Self, Error> {
        let mut conn = connection().await;
        let (jobset, project) = schema::jobsets::table
            .inner_join(schema::projects::table)
            .filter(schema::projects::name.eq(&handle.project.name))
            .filter(schema::jobsets::name.eq(&handle.name))
            .first(&mut *conn)
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

    async fn new_evaluation(&self, url: &String) -> Result<evaluations::Evaluation, Error> {
        use crate::tasks;

        let mut conn = connection().await;

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

        drop(conn);

        let run = {
            let evaluation = evaluation.clone();
            move |sender| evaluation.run(sender)
        };

        let finish = {
            let evaluation = evaluation.clone();
            move |r| evaluation.finish(r)
        };

        evaluation.task.run(run, finish).await?;

        log_event(Event::EvaluationNew(evaluation.handle())).await;

        gcroots::update().await;

        Ok(evaluation)
    }
}
