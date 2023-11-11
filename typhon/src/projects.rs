use crate::actions;
use crate::error::Error;
use crate::gcroots;
use crate::jobsets;
use crate::models;
use crate::nix;
use crate::schema;
use crate::tasks;
use crate::Conn;
use crate::DbPool;
use crate::CURRENT_SYSTEM;
use crate::{handles, responses};
use crate::{log_event, Event};

use typhon_types::data::TaskStatusKind;
use typhon_types::responses::ProjectMetadata;
use typhon_types::*;

use age::secrecy::ExposeSecret;
use diesel::prelude::*;
use serde::Deserialize;
use time::OffsetDateTime;
use tokio::sync::oneshot;

use std::collections::HashMap;
use std::str::FromStr;

#[derive(Clone)]
pub struct Project {
    pub refresh_task: Option<tasks::Task>,
    pub project: models::Project,
}

impl Project {
    pub fn create(
        conn: &mut Conn,
        name: &String,
        decl: &typhon_types::requests::ProjectDecl,
    ) -> Result<(), Error> {
        let handle = handles::project(name.clone());
        if !handle.legal() {
            return Err(Error::IllegalProjectHandle(handle.clone()));
        }
        match Self::get(conn, &handle) {
            Ok(_) => Err(Error::ProjectAlreadyExists(handle.clone())),
            Err(_) => {
                let key = age::x25519::Identity::generate()
                    .to_string()
                    .expose_secret()
                    .clone();
                let new_project = models::NewProject {
                    flake: decl.flake,
                    url: &decl.url,
                    key: &key,
                    name: &handle.name,
                };
                diesel::insert_into(schema::projects::table)
                    .values(&new_project)
                    .execute(conn)?;
                log_event(Event::ProjectNew(handle.clone()));
                Ok(())
            }
        }
    }

    pub fn delete(&self, _conn: &mut Conn) -> Result<(), Error> {
        todo!()
    }

    pub fn get(conn: &mut Conn, handle: &handles::Project) -> Result<Self, Error> {
        let (project, task): (models::Project, Option<models::Task>) = schema::projects::table
            .left_join(schema::tasks::table)
            .filter(schema::projects::name.eq(&handle.name))
            .first(conn)
            .optional()?
            .ok_or(Error::ProjectNotFound(handle.clone()))?;
        Ok(Self {
            refresh_task: task.map(|task| tasks::Task { task }),
            project,
        })
    }

    pub fn handle(&self) -> handles::Project {
        handles::project(self.project.name.clone())
    }

    pub fn info(&self, conn: &mut Conn) -> Result<responses::ProjectInfo, Error> {
        let jobsets_names = schema::jobsets::table
            .filter(schema::jobsets::project_id.eq(&self.project.id))
            .load::<models::Jobset>(conn)?
            .iter()
            .map(|jobset| jobset.name.clone())
            .collect();
        let public_key = age::x25519::Identity::from_str(&self.project.key)
            .map_err(|_| Error::Todo)?
            .to_public()
            .to_string();
        Ok(responses::ProjectInfo {
            actions_path: self.project.actions_path.clone(),
            flake: self.project.flake,
            jobsets: jobsets_names,
            last_refresh: self.refresh_task.clone().map(|task| task.status()),
            metadata: ProjectMetadata {
                title: self.project.title.clone(),
                description: self.project.description.clone(),
                homepage: self.project.homepage.clone(),
            },
            public_key,
            url: self.project.url.clone(),
            url_locked: self.project.url_locked.clone(),
        })
    }

    pub fn list(conn: &mut Conn) -> Result<Vec<(String, ProjectMetadata)>, Error> {
        Ok(schema::projects::table
            .order(schema::projects::name.asc())
            .load::<models::Project>(conn)?
            .iter()
            .map(|project| {
                (
                    project.name.clone(),
                    ProjectMetadata {
                        title: project.title.clone(),
                        description: project.description.clone(),
                        homepage: project.homepage.clone(),
                    },
                )
            })
            .collect())
    }

    pub fn new_action(
        &self,
        conn: &mut Conn,
        path: &String,
        name: &String,
        input: &serde_json::Value,
    ) -> Result<actions::Action, Error> {
        let (action, task) =
            conn.transaction::<(models::Action, tasks::Task), Error, _>(|conn| {
                let task = tasks::Task::new(conn)?;
                let time_created = OffsetDateTime::now_utc().unix_timestamp();
                let max = schema::actions::table
                    .filter(schema::actions::project_id.eq(self.project.id))
                    .select(diesel::dsl::max(schema::actions::num))
                    .first::<Option<i64>>(conn)?
                    .unwrap_or(0);
                let num = max + 1;
                let new_action = models::NewAction {
                    input: &input.to_string(),
                    name,
                    num,
                    path,
                    project_id: self.project.id,
                    task_id: task.task.id,
                    time_created,
                };
                let action = diesel::insert_into(schema::actions::table)
                    .values(&new_action)
                    .get_result::<models::Action>(conn)?;

                Ok((action, task))
            })?;
        Ok(actions::Action {
            project: self.project.clone(),
            action,
            task,
        })
    }

    pub fn refresh(&self, conn: &mut Conn) -> Result<(), Error> {
        #[derive(Deserialize)]
        struct TyphonProject {
            actions: Option<HashMap<String, String>>,
            #[serde(default)]
            meta: ProjectMetadata,
        }

        let run = {
            let url = self.project.url.clone();
            let flake = self.project.flake;
            move |sender| async move {
                let url_locked = nix::lock(&url)?;

                let TyphonProject { actions, meta } =
                    serde_json::from_value(nix::eval(&url_locked, &"typhonProject", flake).await?)
                        .expect("TODO");

                let actions: Option<&String> =
                    actions.as_ref().map(|m| m.get(&*CURRENT_SYSTEM)).flatten();

                let actions_path = if let Some(x) = actions {
                    let drv = nix::derivation(nix::Expr::Path(x.clone())).await?;
                    // FIXME: this should spawn a build
                    Some(nix::build(&drv.path, sender).await?["out"].clone())
                    // TODO: check public key used to encrypt secrets
                } else {
                    None
                };

                Ok((url_locked, meta, actions_path))
            }
        };

        let finish = {
            let self_ = self.clone();
            move |res: Option<Result<(String, ProjectMetadata, Option<String>), Error>>,
                  pool: &DbPool| {
                let status = match res {
                    Some(Ok(x)) => self_.finish_refresh(x, pool),
                    Some(Err(e)) => {
                        log::warn!("refresh error for project {}: {}", self_.handle(), e);
                        Ok(TaskStatusKind::Error)
                    }
                    None => Ok(TaskStatusKind::Canceled),
                };
                log_event(Event::ProjectUpdated(self_.handle()));
                status.unwrap_or(TaskStatusKind::Error)
            }
        };

        let task = tasks::Task::new(conn)?;
        diesel::update(&self.project)
            .set(schema::projects::last_refresh_task_id.eq(task.task.id))
            .execute(conn)?;

        log_event(Event::ProjectUpdated(self.handle()));

        task.run(conn, run, finish)?;

        Ok(())
    }

    pub fn set_decl(
        &self,
        conn: &mut Conn,
        decl: &typhon_types::requests::ProjectDecl,
    ) -> Result<(), Error> {
        diesel::update(&self.project)
            .set((
                schema::projects::flake.eq(decl.flake),
                schema::projects::url.eq(&decl.url),
            ))
            .execute(conn)?;
        log_event(Event::ProjectUpdated(self.handle()));
        Ok(())
    }

    pub fn set_private_key(&self, conn: &mut Conn, key: &String) -> Result<(), Error> {
        let _ = age::x25519::Identity::from_str(key).map_err(|_| Error::Todo)?;
        diesel::update(&self.project)
            .set(schema::projects::key.eq(key))
            .execute(conn)?;
        log_event(Event::ProjectUpdated(self.handle()));
        Ok(())
    }

    pub fn update_jobsets(&self, conn: &mut Conn) -> Result<(), Error> {
        // run action `jobsets`
        let action = self.new_action(
            conn,
            &self
                .project
                .actions_path
                .clone()
                .unwrap_or("/dev/null".to_string()),
            &"jobsets".to_string(),
            &serde_json::Value::Null,
        )?;

        let finish = {
            let self_ = self.clone();
            move |output: Option<String>, pool: &DbPool| {
                let status = match output {
                    Some(output) => {
                        let decls: Result<HashMap<String, jobsets::JobsetDecl>, Error> =
                            serde_json::from_str(&output).map_err(|_| Error::BadJobsetDecl(output));
                        match decls {
                            Ok(decls) => {
                                if self_.finish_update_jobsets(decls, pool).is_ok() {
                                    TaskStatusKind::Success
                                } else {
                                    TaskStatusKind::Error
                                }
                            }
                            Err(_) => TaskStatusKind::Error,
                        }
                    }
                    None => TaskStatusKind::Canceled,
                };
                log_event(Event::ProjectUpdated(self_.handle()));
                status
            }
        };

        action.spawn(conn, finish)?;

        Ok(())
    }

    pub fn webhook(
        &self,
        conn: &mut Conn,
        input: actions::webhooks::Input,
    ) -> Result<Option<Vec<requests::Request>>, Error> {
        let (sender, receiver) = oneshot::channel();

        let input = serde_json::to_value(input).unwrap();

        let action = self.new_action(
            conn,
            &self
                .project
                .actions_path
                .clone() // FIXME? why do we need this clone?
                .unwrap_or("/dev/null".to_string()),
            &"webhook".to_string(),
            &input,
        )?;

        let finish = {
            let handle = self.handle();
            move |output: Option<String>, _: &DbPool| match output {
                Some(output) => match serde_json::from_str::<actions::webhooks::Output>(&output) {
                    Ok(cmds) => {
                        let cmds = cmds
                            .into_iter()
                            .map(|cmd| cmd.lift(handle.clone()))
                            .collect();
                        let _ = sender.send(Some(cmds));
                        TaskStatusKind::Success
                    }
                    Err(_) => {
                        let _ = sender.send(None);
                        TaskStatusKind::Error
                    }
                },
                None => {
                    let _ = sender.send(None);
                    TaskStatusKind::Error
                }
            }
        };

        action.spawn(conn, finish)?;

        Ok(receiver.blocking_recv().map_err(|_| Error::Todo)?)
    }

    fn finish_refresh(
        &self,
        (url_locked, meta, actions_path): (String, ProjectMetadata, Option<String>),
        pool: &DbPool,
    ) -> Result<TaskStatusKind, Error> {
        let mut conn = pool.get().unwrap();
        diesel::update(&self.project)
            .set((
                schema::projects::actions_path.eq(actions_path),
                schema::projects::description.eq(meta.description),
                schema::projects::homepage.eq(meta.homepage),
                schema::projects::title.eq(meta.title),
                schema::projects::url_locked.eq(url_locked),
            ))
            .execute(&mut conn)?;
        gcroots::update(&mut conn);
        Ok(TaskStatusKind::Success)
    }

    fn finish_update_jobsets(
        &self,
        decls: HashMap<String, jobsets::JobsetDecl>,
        pool: &DbPool,
    ) -> Result<TaskStatusKind, Error> {
        let mut conn = pool.get().unwrap();
        let mut current_jobsets: Vec<jobsets::Jobset> = schema::jobsets::table
            .filter(schema::jobsets::project_id.eq(&self.project.id))
            .load::<models::Jobset>(&mut conn)?
            .drain(..)
            .map(|jobset| jobsets::Jobset {
                project: self.project.clone(),
                jobset,
            })
            .collect();

        // delete obsolete jobsets
        let mut set = std::collections::HashSet::<String>::new();
        for jobset in current_jobsets.drain(..) {
            if decls
                .get(&jobset.jobset.name)
                .is_some_and(|decl| *decl == jobset.decl())
            {
                set.insert(jobset.jobset.name);
            } else {
                jobset.delete(&mut conn)?;
            }
        }

        // create new jobsets
        for (name, decl) in decls.iter() {
            if !set.contains(name) {
                let new_jobset = models::NewJobset {
                    flake: decl.flake,
                    name,
                    project_id: self.project.id,
                    url: &decl.url,
                };
                diesel::insert_into(schema::jobsets::table)
                    .values(&new_jobset)
                    .execute(&mut conn)?;
            }
        }

        gcroots::update(&mut conn);

        Ok(TaskStatusKind::Success)
    }
}
