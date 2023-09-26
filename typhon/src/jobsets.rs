use crate::connection;
use crate::error::Error;
use crate::gcroots;
use crate::models::*;
use crate::nix;
use crate::schema::evaluations::dsl::*;
use crate::schema::jobsets::dsl::*;
use crate::schema::projects::dsl::*;
use crate::time;
use crate::{handles, responses};
use crate::{log_event, Event};

use diesel::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct JobsetDecl {
    pub flake: String,
}

impl Jobset {
    pub async fn evaluate(&self, force: bool) -> Result<handles::Evaluation, Error> {
        let project = self.project().await?;

        let flake_locked = nix::lock(&self.jobset_flake).await?;

        let mut conn = connection().await;
        let evaluation = conn.transaction::<Evaluation, Error, _>(|conn| {
            let old_evaluations = evaluations
                .filter(evaluation_jobset.eq(self.jobset_id))
                .load::<Evaluation>(conn)?;
            if !force {
                match old_evaluations.last() {
                    Some(eval) => {
                        if eval.evaluation_flake_locked == flake_locked {
                            return Ok(eval.clone());
                        }
                    }
                    None => (),
                }
            }
            let n = old_evaluations.len() as i32 + 1;
            let status = "pending".to_string();
            let time = time::timestamp();
            let new_evaluation = NewEvaluation {
                evaluation_actions_path: project.project_actions_path.as_ref().map(|s| s.as_str()),
                evaluation_flake_locked: &flake_locked,
                evaluation_jobset: self.jobset_id,
                evaluation_num: n,
                evaluation_status: &status,
                evaluation_time_created: time,
            };
            Ok(diesel::insert_into(evaluations)
                .values(&new_evaluation)
                .get_result(&mut *conn)?)
        })?;
        gcroots::update(&mut *conn);
        drop(conn);

        let handle = evaluation.handle().await?;
        log_event(Event::EvaluationNew(handle.clone()));
        evaluation.run().await;

        Ok(handle)
    }

    pub async fn get(jobset_handle: &handles::Jobset) -> Result<Self, Error> {
        let handles::pattern!(project_name_, jobset_name_) = jobset_handle;
        let project = Project::get(&jobset_handle.project).await?;
        let mut conn = connection().await;
        Ok(jobsets
            .filter(jobset_project.eq(project.project_id))
            .filter(jobset_name.eq(jobset_name_))
            .first::<Jobset>(&mut *conn)
            .map_err(|_| {
                Error::JobsetNotFound(handles::jobset((
                    project_name_.to_string(),
                    jobset_name_.to_string(),
                )))
            })?)
    }

    pub async fn handle(&self) -> Result<handles::Jobset, Error> {
        Ok(handles::Jobset {
            project: self.project().await?.handle(),
            jobset: self.jobset_name.clone(),
        })
    }

    pub async fn info(&self) -> Result<responses::JobsetInfo, Error> {
        let mut conn = connection().await;
        let evals = evaluations
            .filter(evaluation_jobset.eq(self.jobset_id))
            .order(evaluation_id.desc())
            .load::<Evaluation>(&mut *conn)?
            .iter()
            .map(|evaluation| {
                (
                    evaluation.evaluation_num,
                    evaluation.evaluation_time_created,
                )
            })
            .collect();
        drop(conn);
        Ok(responses::JobsetInfo {
            flake: self.jobset_flake.clone(),
            evaluations: evals,
        })
    }

    pub async fn project(&self) -> Result<Project, Error> {
        let mut conn = connection().await;
        Ok(projects
            .find(self.jobset_project)
            .first::<Project>(&mut *conn)?)
    }
}
