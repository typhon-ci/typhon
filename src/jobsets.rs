use crate::connection;
use crate::error::Error;
use crate::models::*;
use crate::nix;
use crate::schema::evaluations::dsl::*;
use crate::schema::jobsets::dsl::*;
use crate::schema::projects::dsl::*;
use crate::time;
use diesel::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct JobsetHandle {
    pub project: String,
    pub jobset: String,
}

impl std::fmt::Display for JobsetHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}", self.project, self.jobset)
    }
}

#[derive(Debug, Serialize)]
pub struct JobsetInfo {
    pub flake: String,
    pub evaluations: Vec<(i32, i64)>,
}

#[derive(Deserialize)]
pub struct JobsetDecl {
    pub flake: String,
}

impl Jobset {
    pub fn evaluate(&self) -> Result<i32, Error> {
        let project = self.project()?;
        let locked_flake = nix::lock(&self.jobset_flake)?;
        let conn = &mut connection();
        let evaluation = conn.transaction::<Evaluation, Error, _>(|conn| {
            let old_evaluations = evaluations
                .filter(evaluation_jobset.eq(self.jobset_id))
                .load::<Evaluation>(conn)?;
            let n = old_evaluations
                .last()
                .map_or(1, |eval| eval.evaluation_num + 1);
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

        let num = evaluation.evaluation_num;
        evaluation.run();

        Ok(num)
    }

    pub fn get(project_name_: &String, jobset_name_: &String) -> Result<Self, Error> {
        let conn = &mut connection();
        let project = Project::get(project_name_)?;
        Ok(jobsets
            .filter(jobset_project.eq(project.project_id))
            .filter(jobset_name.eq(jobset_name_))
            .first::<Jobset>(conn)
            .map_err(|_| {
                Error::JobsetNotFound(JobsetHandle {
                    project: project_name_.to_string(),
                    jobset: jobset_name_.to_string(),
                })
            })?)
    }

    pub fn info(&self) -> Result<JobsetInfo, Error> {
        let conn = &mut connection();
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
        Ok(JobsetInfo {
            flake: self.jobset_flake.clone(),
            evaluations: evals,
        })
    }

    pub fn project(&self) -> Result<Project, Error> {
        let conn = &mut connection();
        Ok(projects.find(self.jobset_project).first::<Project>(conn)?)
    }
}
