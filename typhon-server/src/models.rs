use crate::schema::{builds, evaluations, jobs, jobsets, projects};
use diesel::prelude::*;

#[derive(Queryable, Clone)]
pub struct Project {
    pub project_id: i32,
    pub project_name: String,
    pub project_title: String,
    pub project_description: String,
    pub project_homepage: String,
    pub project_decl: String,
    pub project_decl_locked: String,
    pub project_actions_path: String,
    pub project_key: String,
}

#[derive(Insertable)]
#[diesel(table_name = projects)]
pub struct NewProject<'a> {
    pub project_name: &'a str,
    pub project_key: &'a str,
}

#[derive(Queryable, Clone)]
pub struct Jobset {
    pub jobset_project: i32,
    pub jobset_id: i32,
    pub jobset_name: String,
    pub jobset_flake: String,
}

#[derive(Insertable)]
#[diesel(table_name = jobsets)]
pub struct NewJobset<'a> {
    pub jobset_project: i32,
    pub jobset_name: &'a str,
    pub jobset_flake: &'a str,
}

#[derive(Queryable, Clone)]
pub struct Evaluation {
    pub evaluation_id: i32,
    pub evaluation_jobset: i32,
    pub evaluation_num: i32,
    pub evaluation_locked_flake: String,
    pub evaluation_time_created: i64,
    pub evaluation_actions_path: String,
    pub evaluation_status: String,
}

#[derive(Insertable)]
#[diesel(table_name = evaluations)]
pub struct NewEvaluation<'a> {
    pub evaluation_jobset: i32,
    pub evaluation_num: i32,
    pub evaluation_locked_flake: &'a str,
    pub evaluation_time_created: i64,
    pub evaluation_status: &'a str,
    pub evaluation_actions_path: &'a str,
}

#[derive(Queryable, Clone)]
pub struct Job {
    pub job_evaluation: i32,
    pub job_id: i32,
    pub job_name: String,
    pub job_build: i32,
    pub job_status: String,
}

#[derive(Insertable)]
#[diesel(table_name = jobs)]
pub struct NewJob<'a> {
    pub job_evaluation: i32,
    pub job_name: &'a str,
    pub job_build: i32,
    pub job_status: &'a str,
}

#[derive(Queryable, Clone)]
pub struct Build {
    pub build_id: i32,
    pub build_hash: String,
    pub build_drv: String,
    pub build_status: String,
}

#[derive(Insertable)]
#[diesel(table_name = builds)]
pub struct NewBuild<'a> {
    pub build_hash: &'a str,
    pub build_drv: &'a str,
    pub build_status: &'a str,
}
