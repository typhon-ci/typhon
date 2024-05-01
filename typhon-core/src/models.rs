use crate::schema;
use crate::schema::actions;
use crate::schema::builds;
use crate::schema::evaluations;
use crate::schema::jobs;
use crate::schema::jobsets;
use crate::schema::logs;
use crate::schema::projects;
use crate::schema::runs;
use crate::schema::tasks;

use diesel::backend::Backend;
use diesel::prelude::*;

#[derive(Clone, Debug, Queryable, Selectable, Identifiable)]
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

#[derive(Clone, Debug, Queryable, Selectable)]
pub struct ProjectWithRefreshTask {
    #[diesel(embed)]
    pub project: Project,
    #[diesel(embed)]
    pub last_refresh: Option<Task>,
}

#[derive(Clone, Debug, Queryable, Selectable, Identifiable)]
#[diesel(table_name = jobsets)]
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

#[derive(Clone, Debug, Queryable, Selectable, Identifiable)]
#[diesel(table_name = evaluations)]
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

#[derive(Clone, Debug, Queryable, Selectable, Identifiable)]
#[diesel(table_name = jobs)]
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

#[derive(Clone, Debug, Queryable, Selectable, Identifiable)]
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

#[derive(Clone, Debug, Queryable, Selectable, Identifiable)]
#[diesel(table_name = tasks)]
pub struct Task {
    pub id: i32,
    pub log_id: i32,
    pub status: i32,
    pub time_finished: Option<i64>,
    pub time_started: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct TaskStatus(typhon_types::responses::TaskStatus);

impl<DB: Backend> Selectable<DB> for TaskStatus {
    type SelectExpression = (tasks::status, tasks::time_finished, tasks::time_started);
    fn construct_selection() -> Self::SelectExpression {
        (tasks::status, tasks::time_finished, tasks::time_started)
    }
}

impl
    Queryable<
        <<TaskStatus as Selectable<diesel::sqlite::Sqlite>>::SelectExpression as Expression>::SqlType,
        diesel::sqlite::Sqlite,
    > for TaskStatus
{
    type Row = (i32, Option<i64>, Option<i64>);
    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        let from_timestamp = |t| time::OffsetDateTime::from_unix_timestamp(t).unwrap();
        Ok(Self(
            typhon_types::responses::TaskStatusKind::try_from(row.0)?
                .into_task_status(row.1.map(from_timestamp), row.2.map(from_timestamp)),
        ))
    }
}

#[derive(Debug, Insertable)]
#[diesel(table_name = tasks)]
pub struct NewTask {
    pub log_id: i32,
    pub status: i32,
}

#[derive(Clone, Debug, Queryable, Selectable, Identifiable)]
#[diesel(table_name = builds)]
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

#[derive(Clone, Debug, Queryable, Selectable, Identifiable)]
#[diesel(table_name = actions)]
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

#[derive(Clone, Debug, Queryable)]
pub struct ActionPlus {
    pub action: Action,
    pub key: String,
    pub project_name: String,
}

impl<DB: Backend> Selectable<DB> for ActionPlus {
    type SelectExpression = (
        <Action as Selectable<DB>>::SelectExpression,
        schema::projects::key,
        schema::projects::name,
    );
    fn construct_selection() -> Self::SelectExpression {
        (
            <Action as Selectable<DB>>::construct_selection(),
            schema::projects::key,
            schema::projects::name,
        )
    }
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

#[derive(Clone, Debug, Queryable, Selectable, Identifiable)]
#[diesel(table_name = runs)]
pub struct Run {
    pub begin_id: Option<i32>,
    pub build_id: Option<i32>,
    pub end_id: Option<i32>,
    pub id: i32,
    pub job_id: i32,
    pub num: i32,
    pub time_created: i64,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = runs)]
pub struct NewRun {
    pub job_id: i32,
    pub num: i32,
    pub time_created: i64,
}
