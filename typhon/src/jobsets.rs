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
        let evaluations: Vec<evaluations::Evaluation> = schema::evaluations::table
            .filter(schema::evaluations::jobset_id.eq(&self.jobset.id))
            .load::<models::Evaluation>(&mut *conn)?
            .drain(..)
            .map(|evaluation| evaluations::Evaluation {
                evaluation,
                jobset: self.jobset.clone(),
                project: self.project.clone(),
            })
            .collect();
        drop(conn);

        for evaluation in evaluations.iter() {
            evaluation.delete().await?;
        }

        let mut conn = connection().await;
        diesel::delete(schema::jobsets::table.find(&self.jobset.id)).execute(&mut *conn)?;
        drop(conn);

        Ok(())
    }

    pub async fn evaluate(&self, force: bool) -> Result<handles::Evaluation, Error> {
        let url = nix::lock(&self.jobset.url).await?;

        let mut conn = connection().await;

        let evaluation = conn.transaction::<models::Evaluation, Error, _>(|conn| {
            // check for an existing evaluation
            let preexisting_eval = schema::evaluations::table
                .filter(schema::evaluations::jobset_id.eq(self.jobset.id))
                .filter(schema::evaluations::url.eq(&url))
                .first::<models::Evaluation>(conn)
                .optional()?;
            let max = schema::evaluations::table
                .select(diesel::dsl::max(schema::evaluations::num))
                .first::<Option<i64>>(conn)?
                .unwrap_or(0);

            // continue if the evaluation is forced
            if let Some(eval) = preexisting_eval {
                if !force {
                    return Ok(eval);
                }
            }

            // create a new evaluation
            let num = max + 1;
            let status = "pending".to_string();
            let time_created = crate::time::now();
            let new_log = models::NewLog { stderr: None };
            let log = diesel::insert_into(schema::logs::dsl::logs)
                .values(&new_log)
                .get_result::<models::Log>(conn)?;
            let new_evaluation = models::NewEvaluation {
                actions_path: self.project.actions_path.as_ref().map(|s| s.as_str()),
                jobset_id: self.jobset.id,
                log_id: log.id,
                num,
                status: &status,
                time_created,
                url: &url,
            };
            let evaluation = diesel::insert_into(schema::evaluations::table)
                .values(&new_evaluation)
                .get_result::<models::Evaluation>(conn)?;

            Ok(evaluation)
        })?;

        drop(conn);

        gcroots::update().await;

        let evaluation = evaluations::Evaluation {
            project: self.project.clone(),
            jobset: self.jobset.clone(),
            evaluation,
        };
        log_event(Event::EvaluationNew(evaluation.handle())).await;
        evaluation.run().await?;

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

    pub async fn info(&self) -> Result<responses::JobsetInfo, Error> {
        let mut conn = connection().await;
        let last_evaluation = schema::evaluations::table
            .filter(schema::evaluations::jobset_id.eq(self.jobset.id))
            .first::<models::Evaluation>(&mut *conn)
            .optional()?
            .map(|eval| (eval.num, eval.time_created));
        Ok(responses::JobsetInfo {
            last_evaluation,
            flake: self.jobset.flake,
            url: self.jobset.url.clone(),
        })
    }
}
