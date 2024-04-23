use crate::error::Error;
use crate::evaluations;
use crate::gcroots;
use crate::models;
use crate::nix;
use crate::schema;
use crate::Conn;
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

    pub fn handle(&self) -> handles::Jobset {
        handles::Jobset {
            project: handles::Project {
                name: self.project.name.clone(),
            },
            name: self.jobset.name.clone(),
        }
    }

    pub fn info(&self) -> responses::JobsetInfo {
        responses::JobsetInfo {
            handle: self.handle(),
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
        use uuid::{timestamp, Uuid};

        let evaluation = conn.transaction::<evaluations::Evaluation, Error, _>(|conn| {
            let task = tasks::Task::new(conn)?;
            let time_created = OffsetDateTime::now_utc().unix_timestamp();
            let uuid = Uuid::new_v7(timestamp::Timestamp::from_unix(
                timestamp::context::NoContext,
                time_created as u64,
                0,
            ));
            let new_evaluation = models::NewEvaluation {
                actions_path: self.project.actions_path.as_ref().map(|s| s.as_str()),
                flake: self.jobset.flake,
                jobset_name: &self.jobset.name,
                project_id: self.project.id,
                task_id: task.task.id,
                time_created,
                url: &url,
                uuid: &uuid.to_string(),
            };
            let evaluation = diesel::insert_into(schema::evaluations::table)
                .values(&new_evaluation)
                .get_result::<models::Evaluation>(conn)?;
            Ok(evaluations::Evaluation {
                project: self.project.clone(),
                evaluation,
                task,
            })
        })?;

        let run = {
            let evaluation = evaluation.clone();
            move |sender| evaluation.run(sender)
        };

        let finish = {
            let evaluation = evaluation.clone();
            move |r| {
                let handle = evaluation.handle();
                let status = evaluation.finish(r);
                (status, Event::EvaluationFinished(handle))
            }
        };

        log_event(Event::EvaluationNew(evaluation.handle()));

        evaluation.task.run(conn, run, finish)?;

        gcroots::update(conn);

        Ok(evaluation)
    }
}
