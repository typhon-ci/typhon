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
use crate::DbPool;
use crate::RUNS;

use typhon_types::data::TaskStatusKind;
use typhon_types::*;

use diesel::prelude::*;

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
    pub fn cancel(&self) {
        RUNS.cancel(self.run.id);
    }

    pub fn get(conn: &mut Conn, handle: &handles::Run) -> Result<Self, Error> {
        let (run, (job, (evaluation, project))): (
            models::Run,
            (models::Job, (models::Evaluation, models::Project)),
        ) = schema::runs::table
            .inner_join(
                schema::jobs::table
                    .inner_join(schema::evaluations::table.inner_join(schema::projects::table)),
            )
            .filter(schema::projects::name.eq(&handle.job.evaluation.project.name))
            .filter(schema::evaluations::num.eq(handle.job.evaluation.num as i64))
            .filter(schema::jobs::system.eq(&handle.job.system))
            .filter(schema::jobs::name.eq(&handle.job.name))
            .filter(schema::runs::num.eq(handle.num as i64))
            .first(conn)
            .optional()?
            .ok_or(Error::RunNotFound(handle.clone()))?;
        let build = schema::builds::table
            .inner_join(schema::tasks::table)
            .filter(schema::builds::id.nullable().eq(run.build_id))
            .first(conn)
            .optional()?
            .map(|(build, task)| builds::Build {
                task: tasks::Task { task },
                build,
            });
        let mut get_action = |id| {
            schema::actions::table
                .inner_join(schema::tasks::table)
                .filter(schema::actions::id.nullable().eq(id))
                .first(conn)
                .optional()
        };
        let into_action = |(action, task)| actions::Action {
            task: tasks::Task { task },
            action,
            project: project.clone(),
        };
        Ok(Self {
            begin: get_action(run.begin_id)?.map(into_action),
            end: get_action(run.end_id)?.map(into_action),
            build,
            run,
            job,
            evaluation,
            project,
        })
    }

    pub fn handle(&self) -> handles::Run {
        handles::run((
            self.project.name.clone(),
            self.evaluation.num as u64,
            self.job.system.clone(),
            self.job.name.clone(),
            self.run.num as u64,
        ))
    }

    pub fn info(&self) -> responses::RunInfo {
        responses::RunInfo {
            begin: self.begin.as_ref().map(|action| action.handle()),
            end: self.end.as_ref().map(|action| action.handle()),
            build: self.build.as_ref().map(|build| build.handle()),
        }
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
                Some(None) => TaskStatusKind::Error,
                None => TaskStatusKind::Canceled,
            }
        };

        // run the 'end' action
        let finish_run = {
            let self_ = self.clone();
            let finish_err = move |status, pool: &DbPool| {
                if let Some(status) = status {
                    let mut conn = pool.get().unwrap();
                    let action_end = self_.spawn_action(&mut conn, "end", status)?;
                    diesel::update(&self_.run)
                        .set((schema::runs::end_id.eq(action_end.action.id),))
                        .execute(&mut conn)?;
                    log_event(Event::RunUpdated(self_.handle()));
                }
                Ok::<_, Error>(())
            };
            move |status, pool: &DbPool| {
                finish_err(status, pool).unwrap(); // FIXME
            }
        };

        RUNS.run(self.run.id, run_run, finish_run);

        Ok(())
    }

    fn mk_input(&self, status: TaskStatusKind) -> Result<serde_json::Value, Error> {
        Ok(serde_json::json!({
            "drv": self.job.drv,
            "evaluation": self.evaluation.num,
            "flake": self.evaluation.flake,
            "job": self.job.name,
            "jobset": self.evaluation.jobset_name,
            "out": self.job.out,
            "project": self.project.name,
            "status": status.to_string(),
            "system": self.job.system,
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

        let finish = move |res, _: &DbPool| match res {
            Some(_) => TaskStatusKind::Success,
            None => TaskStatusKind::Error,
        };

        action.spawn(conn, finish)?;

        Ok(action)
    }
}
