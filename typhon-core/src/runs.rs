use crate::actions;
use crate::builds;
use crate::error::Error;
use crate::handles;
use crate::log_event;
use crate::models;
use crate::responses;
use crate::schema;
use crate::tasks;
use crate::Conn;
use crate::POOL;
use crate::RUNS;

use typhon_types::data::TaskStatusKind;
use typhon_types::*;

use diesel::prelude::*;
use uuid::Uuid;

use std::str::FromStr;

#[derive(Clone)]
pub struct Run {
    pub begin: Option<actions::Action>,
    pub end: Option<actions::Action>,
    pub build: Option<builds::Build>,
    pub run: models::Run,
    pub job: models::Job,
    pub evaluation: models::Evaluation,
    pub project: models::Project,
}

impl Run {
    //pub fn cancel(&self) {
    //    RUNS.cancel(self.run.id);
    //}

    pub fn get(conn: &mut Conn, handle: &handles::Run) -> Result<Self, Error> {
        let (begin_action, end_action, begin_task, build_task, end_task) = diesel::alias!(
            schema::actions as begin_action,
            schema::actions as end_action,
            schema::tasks as begin_task,
            schema::tasks as build_task,
            schema::tasks as end_task,
        );
        let (job, evaluation, project, run, begin, build, end) = schema::runs::table
            .inner_join(
                schema::jobs::table
                    .inner_join(schema::evaluations::table.inner_join(schema::projects::table)),
            )
            .left_join(
                begin_action
                    .on(begin_action
                        .field(schema::actions::id)
                        .nullable()
                        .eq(schema::runs::begin_id))
                    .inner_join(begin_task),
            )
            .left_join(
                schema::builds::table
                    .on(schema::builds::id.nullable().eq(schema::runs::build_id))
                    .inner_join(build_task),
            )
            .left_join(
                end_action
                    .on(end_action
                        .field(schema::actions::id)
                        .nullable()
                        .eq(schema::runs::end_id))
                    .inner_join(end_task),
            )
            .filter(
                schema::evaluations::uuid.eq(handle
                    .job
                    .evaluation
                    .uuid
                    .as_hyphenated()
                    .to_string()),
            )
            .filter(schema::jobs::name.eq(&handle.job.name))
            .filter(schema::runs::num.eq(handle.num as i32))
            .select((
                schema::jobs::all_columns,
                schema::evaluations::all_columns,
                schema::projects::all_columns,
                schema::runs::all_columns,
                (
                    begin_action.fields(schema::actions::all_columns),
                    begin_task.fields(schema::tasks::all_columns),
                )
                    .nullable(),
                (
                    schema::builds::all_columns,
                    build_task.fields(schema::tasks::all_columns),
                )
                    .nullable(),
                (
                    end_action.fields(schema::actions::all_columns),
                    end_task.fields(schema::tasks::all_columns),
                )
                    .nullable(),
            ))
            .first::<(
                models::Job,
                models::Evaluation,
                models::Project,
                models::Run,
                Option<(models::Action, models::Task)>,
                Option<(models::Build, models::Task)>,
                Option<(models::Action, models::Task)>,
            )>(conn)
            .optional()?
            .ok_or(Error::RunNotFound(handle.clone()))?;
        Ok(Run {
            begin: begin.map(|(action, task)| actions::Action {
                project: project.clone(),
                action,
                task: tasks::Task { task },
            }),
            build: build.map(|(build, task)| builds::Build {
                build,
                task: tasks::Task { task },
            }),
            end: end.map(|(action, task)| actions::Action {
                project: project.clone(),
                action,
                task: tasks::Task { task },
            }),
            run,
            job,
            evaluation,
            project,
        })
    }

    pub fn handle(&self) -> handles::Run {
        handles::run((
            Uuid::from_str(&self.evaluation.uuid).unwrap(),
            self.job.name.clone(),
            self.run.num as u32,
        ))
    }

    pub fn info(&self) -> responses::RunInfo {
        use crate::evaluations::ExtraRunInfo;
        let Run {
            run,
            begin,
            build,
            end,
            ..
        } = self.clone();
        responses::RunInfo::new(
            &handles::project(self.project.name.clone()),
            &self.handle().job,
            run,
            begin.map(|actions::Action { action, task, .. }| (action, task.task)),
            build.map(|builds::Build { build, task }| (build, task.task)),
            end.map(|actions::Action { action, task, .. }| (action, task.task)),
        )
    }

    pub fn run(&self, conn: &mut Conn) -> Result<(), Error> {
        use crate::build_manager::BUILDS;
        use crate::nix;
        use crate::TASKS;

        // run the build
        let drv = nix::DrvPath::new(&self.job.drv);
        let build_handle = BUILDS.run(drv);

        // run the 'begin' action
        let action_begin = self.spawn_action(conn, "begin", TaskStatusKind::Pending)?;

        diesel::update(&self.run)
            .set((
                schema::runs::begin_id.eq(action_begin.action.id),
                schema::runs::build_id.eq(build_handle.id),
            ))
            .execute(conn)?;
        log_event(Event::RunUpdated(self.handle()));

        // a waiter task
        let run_run = async move {
            TASKS.wait(&action_begin.task.task.id).await;
            let res = build_handle.wait().await;
            match res {
                Some(Some(())) => TaskStatusKind::Success,
                Some(None) => TaskStatusKind::Failure,
                None => TaskStatusKind::Canceled,
            }
        };

        // run the 'end' action
        let finish_run = {
            let self_ = self.clone();
            let finish_err = move |status| {
                if let Some(status) = status {
                    let mut conn = POOL.get().unwrap();
                    let action_end = self_.spawn_action(&mut conn, "end", status)?;
                    diesel::update(&self_.run)
                        .set((schema::runs::end_id.eq(action_end.action.id),))
                        .execute(&mut conn)?;
                    log_event(Event::RunUpdated(self_.handle()));
                }
                Ok::<_, Error>(())
            };
            move |status| {
                finish_err(status).unwrap(); // FIXME
                None::<()>
            }
        };

        RUNS.run(self.run.id, (run_run, finish_run));

        Ok(())
    }

    fn mk_input(&self, status: TaskStatusKind) -> Result<serde_json::Value, Error> {
        Ok(serde_json::json!({
            "drv": self.job.drv,
            "evaluation": self.evaluation.uuid,
            "flake": self.evaluation.flake,
            "job": self.job.name,
            "jobset": self.evaluation.jobset_name,
            "out": self.job.out,
            "project": self.project.name,
            "status": status.to_string(),
            "url": self.evaluation.url,
        }))
    }

    fn spawn_action(
        &self,
        conn: &mut Conn,
        name: &str,
        status: TaskStatusKind,
    ) -> Result<actions::Action, Error> {
        use crate::projects;

        let project = projects::Project {
            refresh_task: None, // FIXME?
            project: self.project.clone(),
        };

        let input = self.mk_input(status)?;

        let action = project.new_action(
            conn,
            &self
                .clone() // FIXME: why do we need this clone?
                .evaluation
                .actions_path
                .unwrap_or("/dev/null".to_string()),
            &name.to_string(),
            &input,
        )?;

        let finish = move |res| match res {
            Some(_) => TaskStatusKind::Success,
            None => TaskStatusKind::Failure,
        };

        action.spawn(conn, finish)?;

        Ok(action)
    }
}
