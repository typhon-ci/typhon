use crate::schema::{evaluations, jobs, jobsets, logs, projects};
use diesel::prelude::*;

#[derive(Queryable, Clone, Identifiable, Selectable)]
#[diesel(table_name = projects)]
pub struct Project {
    pub actions_path: Option<String>,
    pub description: String,
    pub flake: bool,
    pub homepage: String,
    pub id: i32,
    pub key: String,
    pub name: String,
    pub title: String,
    pub url: String,
    pub url_locked: String,
}

#[derive(Insertable)]
#[diesel(table_name = projects)]
pub struct NewProject<'a> {
    pub flake: bool,
    pub key: &'a str,
    pub name: &'a str,
    pub url: &'a str,
}

#[derive(Queryable, Clone, Identifiable, Selectable)]
#[diesel(table_name = jobsets)]
#[diesel(belongs_to(Project))]
pub struct Jobset {
    pub flake: bool,
    pub id: i32,
    pub name: String,
    pub project_id: i32,
    pub url: String,
}

#[derive(Insertable)]
#[diesel(table_name = jobsets)]
pub struct NewJobset<'a> {
    pub flake: bool,
    pub name: &'a str,
    pub project_id: i32,
    pub url: &'a str,
}

#[derive(Queryable, Clone, Identifiable, Selectable)]
#[diesel(table_name = evaluations)]
#[diesel(belongs_to(Project))]
pub struct Evaluation {
    pub actions_path: Option<String>,
    pub flake: bool,
    pub id: i32,
    pub jobset_name: String,
    pub log_id: i32,
    pub num: i64,
    pub project_id: i32,
    pub status: String,
    pub time_created: i64,
    pub time_finished: Option<i64>,
    pub url: String,
}

#[derive(Insertable)]
#[diesel(table_name = evaluations)]
pub struct NewEvaluation<'a> {
    pub actions_path: Option<&'a str>,
    pub flake: bool,
    pub jobset_name: &'a str,
    pub log_id: i32,
    pub num: i64,
    pub project_id: i32,
    pub status: &'a str,
    pub time_created: i64,
    pub url: &'a str,
}

#[derive(Queryable, Clone, Identifiable, Selectable)]
#[diesel(table_name = jobs)]
#[diesel(belongs_to(Evaluation))]
pub struct Job {
    pub begin_log_id: i32,
    pub begin_status: String,
    pub begin_time_finished: Option<i64>,
    pub begin_time_started: Option<i64>,
    pub build_drv: String,
    pub build_out: String,
    pub build_status: String,
    pub build_time_finished: Option<i64>,
    pub build_time_started: Option<i64>,
    pub dist: bool,
    pub end_log_id: i32,
    pub end_status: String,
    pub end_time_finished: Option<i64>,
    pub end_time_started: Option<i64>,
    pub evaluation_id: i32,
    pub id: i32,
    pub name: String,
    pub system: String,
    pub time_created: i64,
}

#[derive(Insertable)]
#[diesel(table_name = jobs)]
pub struct NewJob<'a> {
    pub begin_log_id: i32,
    pub begin_status: &'a str,
    pub build_drv: &'a str,
    pub build_out: &'a str,
    pub build_status: &'a str,
    pub dist: bool,
    pub end_log_id: i32,
    pub end_status: &'a str,
    pub evaluation_id: i32,
    pub name: &'a str,
    pub system: &'a str,
    pub time_created: i64,
}

#[derive(Queryable, Clone, Identifiable, Selectable)]
#[diesel(table_name = logs)]
pub struct Log {
    pub id: i32,
    pub stderr: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = logs)]
pub struct NewLog<'a> {
    pub stderr: Option<&'a str>,
}
