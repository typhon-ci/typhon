use crate::error::Error;
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
    pub async fn evaluate(
        &self,
        conn: &mut SqliteConnection,
        force: bool,
    ) -> Result<handles::Evaluation, Error> {
        let project = self.project(conn)?;

        // TODO: don't block connection during nix call
        let locked_flake = nix::lock(&self.jobset_flake).await?;

        let evaluation = conn.transaction::<Evaluation, Error, _>(|conn| {
            let old_evaluations = evaluations
                .filter(evaluation_jobset.eq(self.jobset_id))
                .load::<Evaluation>(conn)?;
            if !force {
                match old_evaluations.last() {
                    Some(eval) => {
                        if eval.evaluation_locked_flake == locked_flake {
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
                evaluation_num: n,
                evaluation_jobset: self.jobset_id,
                evaluation_locked_flake: &locked_flake,
                evaluation_time_created: time,
                evaluation_status: &status,
                evaluation_actions_path: project.project_actions_path.as_ref().map(|s| s.as_str()),
            };
            Ok(diesel::insert_into(evaluations)
                .values(&new_evaluation)
                .get_result(conn)?)
        })?;

        let handle = evaluation.handle(conn)?;
        log_event(Event::EvaluationNew(handle.clone()));
        evaluation.run(conn).await;

        Ok(handle)
    }

    pub fn get(
        conn: &mut SqliteConnection,
        jobset_handle: &handles::Jobset,
    ) -> Result<Self, Error> {
        let handles::pattern!(project_name_, jobset_name_) = jobset_handle;
        let project = Project::get(conn, &jobset_handle.project)?;
        Ok(jobsets
            .filter(jobset_project.eq(project.project_id))
            .filter(jobset_name.eq(jobset_name_))
            .first::<Jobset>(conn)
            .map_err(|_| {
                Error::JobsetNotFound(handles::jobset((
                    project_name_.to_string(),
                    jobset_name_.to_string(),
                )))
            })?)
    }

    pub fn handle(&self, conn: &mut SqliteConnection) -> Result<handles::Jobset, Error> {
        Ok(handles::Jobset {
            project: self.project(conn)?.handle(),
            jobset: self.jobset_name.clone(),
        })
    }

    pub fn info(&self, conn: &mut SqliteConnection) -> Result<responses::JobsetInfo, Error> {
        let evals = evaluations
            .filter(evaluation_jobset.eq(self.jobset_id))
            .order(evaluation_id.desc())
            .load::<Evaluation>(conn)?
            .iter()
            .map(|evaluation| {
                (
                    evaluation.evaluation_num,
                    evaluation.evaluation_time_created,
                )
            })
            .collect();
        Ok(responses::JobsetInfo {
            flake: self.jobset_flake.clone(),
            evaluations: evals,
        })
    }

    pub fn project(&self, conn: &mut SqliteConnection) -> Result<Project, Error> {
        Ok(projects.find(self.jobset_project).first::<Project>(conn)?)
    }
}
