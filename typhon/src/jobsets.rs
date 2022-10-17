use crate::connection;
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
    pub fn evaluate(&self) -> Result<handles::Evaluation, Error> {
        let project = self.project()?;
        let locked_flake = nix::lock(&self.jobset_flake)?;
        let conn = &mut *connection();
        let evaluation = conn.transaction::<Evaluation, Error, _>(|conn| {
            let old_evaluations = evaluations
                .filter(evaluation_jobset.eq(self.jobset_id))
                .load::<Evaluation>(conn)?;
            let n = old_evaluations.len() as i32 + 1;
            let status = "pending".to_string();
            let time = time::timestamp();
            let new_evaluation = NewEvaluation {
                evaluation_num: n,
                evaluation_jobset: self.jobset_id,
                evaluation_locked_flake: &locked_flake,
                evaluation_time_created: time,
                evaluation_status: &status,
                evaluation_actions_path: &project.project_actions_path,
            };
            Ok(diesel::insert_into(evaluations)
                .values(&new_evaluation)
                .get_result(conn)?)
        })?;

        let handle = evaluation.handle()?;
        log_event(Event::EvaluationNew(handle.clone()));
        evaluation.run();

        Ok(handle)
    }

    pub fn get(jobset_handle: &handles::Jobset) -> Result<Self, Error> {
        let handles::pattern!(project_name_, jobset_name_) = jobset_handle;
        let project = Project::get(&jobset_handle.project)?;
        let conn = &mut *connection();
        Ok(jobsets
            .filter(jobset_project.eq(project.project_id))
            .filter(jobset_name.eq(jobset_name_))
            .first::<Jobset>(conn)
            .map_err(|_| {
                Error::JobsetNotFound(handles::jobset(
                    project_name_.to_string(),
                    jobset_name_.to_string(),
                ))
            })?)
    }

    pub fn handle(&self) -> Result<handles::Jobset, Error> {
        Ok(handles::Jobset {
            project: self.project()?.handle()?,
            jobset: self.jobset_name.clone(),
        })
    }

    pub fn info(&self) -> Result<responses::JobsetInfo, Error> {
        let conn = &mut *connection();
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

    pub fn project(&self) -> Result<Project, Error> {
        let conn = &mut *connection();
        Ok(projects.find(self.jobset_project).first::<Project>(conn)?)
    }
}
