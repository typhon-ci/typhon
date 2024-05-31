use crate::schema::actions;
use crate::schema::builds;
use crate::schema::evaluations;
use crate::schema::jobs;
use crate::schema::jobsets;
use crate::schema::logs;
use crate::schema::projects;
use crate::schema::runs;
use crate::schema::tasks;

use diesel::prelude::*;

#[derive(Debug, Queryable, Clone, Identifiable, Selectable)]
#[diesel(table_name = projects)]
pub struct Project {
    pub actions_path: Option<String>,
    pub description: String,
    pub flake: bool,
    pub homepage: String,
    pub id: i32,
    pub key: String,
    pub last_refresh_task_id: Option<i32>,
    pub name: String,
    pub title: String,
    pub url: String,
    pub url_locked: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = projects)]
pub struct NewProject<'a> {
    pub flake: bool,
    pub key: &'a str,
    pub name: &'a str,
    pub url: &'a str,
}

#[derive(Debug, Queryable, Clone, Identifiable, Selectable)]
#[diesel(table_name = jobsets)]
#[diesel(belongs_to(Project))]
pub struct Jobset {
    pub flake: bool,
    pub id: i32,
    pub name: String,
    pub project_id: i32,
    pub url: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = jobsets)]
pub struct NewJobset<'a> {
    pub flake: bool,
    pub name: &'a str,
    pub project_id: i32,
    pub url: &'a str,
}

#[derive(Debug, Queryable, Clone, Identifiable, Selectable)]
#[diesel(table_name = evaluations)]
#[diesel(belongs_to(Project))]
#[diesel(belongs_to(Task))]
pub struct Evaluation {
    pub actions_path: Option<String>,
    pub flake: bool,
    pub id: i32,
    pub jobset_name: String,
    pub project_id: i32,
    pub task_id: i32,
    pub time_created: i64,
    pub url: String,
    pub uuid: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = evaluations)]
pub struct NewEvaluation<'a> {
    pub actions_path: Option<&'a str>,
    pub flake: bool,
    pub jobset_name: &'a str,
    pub project_id: i32,
    pub task_id: i32,
    pub time_created: i64,
    pub url: &'a str,
    pub uuid: &'a str,
}

#[derive(Debug, Queryable, Clone, Identifiable, Selectable)]
#[diesel(table_name = jobs)]
#[diesel(belongs_to(Evaluation))]
pub struct Job {
    pub dist: bool,
    pub drv: String,
    pub evaluation_id: i32,
    pub id: i32,
    pub name: String,
    pub out: String,
    pub system: String,
    pub tries: i32,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = jobs)]
pub struct NewJob<'a> {
    pub dist: bool,
    pub drv: &'a str,
    pub evaluation_id: i32,
    pub name: &'a str,
    pub out: &'a str,
    pub system: &'a str,
    pub tries: i32,
}

#[derive(Debug, Queryable, Clone, Identifiable, Selectable)]
#[diesel(table_name = logs)]
pub struct Log {
    pub id: i32,
    pub stderr: Option<String>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = logs)]
pub struct NewLog<'a> {
    pub stderr: Option<&'a str>, // FIXME
}

#[derive(Debug, Queryable, Clone, Identifiable, Selectable)]
#[diesel(table_name = tasks)]
#[diesel(belongs_to(Log))]
pub struct Task {
    pub id: i32,
    pub log_id: i32,
    pub status: i32,
    pub time_finished: Option<i64>,
    pub time_started: Option<i64>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = tasks)]
pub struct NewTask {
    pub log_id: i32,
    pub status: i32,
}

#[derive(Debug, Queryable, Clone, Identifiable, Selectable)]
#[diesel(table_name = builds)]
#[diesel(belongs_to(Task))]
pub struct Build {
    pub drv: String,
    pub id: i32,
    pub task_id: i32,
    pub time_created: i64,
    pub uuid: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = builds)]
pub struct NewBuild<'a> {
    pub drv: &'a str,
    pub task_id: i32,
    pub time_created: i64,
    pub uuid: &'a str,
}

#[derive(Debug, Queryable, Clone, Identifiable, Selectable)]
#[diesel(table_name = actions)]
#[diesel(belongs_to(Project))]
#[diesel(belongs_to(Task))]
pub struct Action {
    pub id: i32,
    pub input: String,
    pub name: String,
    pub path: String,
    pub project_id: i32,
    pub task_id: i32,
    pub time_created: i64,
    pub uuid: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = actions)]
pub struct NewAction<'a> {
    pub input: &'a str,
    pub name: &'a str,
    pub path: &'a str,
    pub project_id: i32,
    pub task_id: i32,
    pub time_created: i64,
    pub uuid: &'a str,
}

#[derive(Debug, Queryable, Clone, Identifiable, Selectable)]
#[diesel(table_name = runs)]
#[diesel(belongs_to(Job))]
#[diesel(belongs_to(Task))]
pub struct Run {
    pub begin_id: Option<i32>,
    pub build_id: Option<i32>,
    pub end_id: Option<i32>,
    pub id: i32,
    pub job_id: i32,
    pub num: i32,
    pub task_id: i32,
    pub time_created: i64,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = runs)]
pub struct NewRun {
    pub job_id: i32,
    pub num: i32,
    pub task_id: i32,
    pub time_created: i64,
}
