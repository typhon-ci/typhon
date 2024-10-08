use crate::actions;
use crate::error::Error;
use crate::gcroots;
use crate::jobsets;
use crate::models;
use crate::nix;
use crate::schema;
use crate::tasks;
use crate::Conn;
use crate::CURRENT_SYSTEM;
use crate::POOL;
use crate::{handles, responses};
use crate::{log_event, Event};

use typhon_types::data::TaskStatusKind;
use typhon_types::requests::JobsetDecl;
use typhon_types::responses::ProjectMetadata;

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

    //pub fn delete(&self, _conn: &mut Conn) -> Result<(), Error> {
    //    todo!()
    //}

    pub fn delete_jobset(&self, conn: &mut Conn, name: &String) -> Result<(), Error> {
        let jobset = jobsets::Jobset::get(
            conn,
            &handles::Jobset {
                project: self.handle(),
                name: name.clone(),
            },
        )?;
        diesel::delete(schema::jobsets::table.find(&jobset.jobset.id)).execute(conn)?;
        Ok(())
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
            handle: self.handle(),
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

    pub fn new_action(
        &self,
        conn: &mut Conn,
        path: &String,
        name: &String,
        input: &serde_json::Value,
    ) -> Result<actions::Action, Error> {
        use uuid::{timestamp, Uuid};

        conn.transaction::<actions::Action, Error, _>(|conn| {
            let task = tasks::Task::new(conn)?;
            let time_created = OffsetDateTime::now_utc().unix_timestamp();
            let uuid = Uuid::new_v7(timestamp::Timestamp::from_unix(
                timestamp::context::NoContext,
                time_created as u64,
                0,
            ));
            let new_action = models::NewAction {
                input: &input.to_string(),
                name,
                path,
                project_id: self.project.id,
                task_id: task.task.id,
                time_created,
                uuid: &uuid.to_string(),
            };
            let action = diesel::insert_into(schema::actions::table)
                .values(&new_action)
                .get_result::<models::Action>(conn)?;
            Ok(actions::Action {
                project: self.project.clone(),
                action,
                task,
            })
        })
    }

    pub fn new_jobset(
        &self,
        conn: &mut Conn,
        name: &String,
        decl: &JobsetDecl,
    ) -> Result<(), Error> {
        let new_jobset = models::NewJobset {
            flake: decl.flake,
            name,
            project_id: self.project.id,
            url: &decl.url,
        };
        diesel::insert_into(schema::jobsets::table)
            .values(&new_jobset)
            .execute(conn)?;
        Ok(())
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
                        .map_err(|_| Error::BadProjectDecl)?;

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
            move |res: Option<Result<(String, ProjectMetadata, Option<String>), Error>>| {
                let status = match res {
                    Some(Ok(x)) => self_.finish_refresh(x),
                    Some(Err(e)) => {
                        tracing::warn!("refresh error for project {}: {}", self_.handle(), e);
                        Ok(TaskStatusKind::Failure)
                    }
                    None => Ok(TaskStatusKind::Canceled),
                };
                (
                    status.unwrap_or(TaskStatusKind::Failure),
                    Event::ProjectUpdated(self_.handle()),
                )
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
            move |output: Option<String>| {
                let status = match output {
                    Some(output) => {
                        let decls: Result<HashMap<String, JobsetDecl>, Error> =
                            serde_json::from_str(&output).map_err(|_| Error::BadJobsetDecl(output));
                        match decls {
                            Ok(decls) => {
                                if self_.finish_update_jobsets(decls).is_ok() {
                                    TaskStatusKind::Success
                                } else {
                                    TaskStatusKind::Failure
                                }
                            }
                            Err(_) => TaskStatusKind::Failure,
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

    pub fn webhook(&self, conn: &mut Conn, input: actions::webhooks::Input) -> Result<(), Error> {
        use crate::handle_request_aux;
        use crate::User;

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

        let finish = move |output: Option<String>| match output {
            Some(output) => match serde_json::from_str::<actions::webhooks::Output>(&output) {
                Ok(cmds) => {
                    let _ = sender.send(Ok(cmds));
                    TaskStatusKind::Success
                }
                Err(_) => {
                    let _ = sender.send(Err(Some(output)));
                    TaskStatusKind::Failure
                }
            },
            None => {
                let _ = sender.send(Err(None));
                TaskStatusKind::Failure
            }
        };

        action.spawn(conn, finish)?;

        let cmds = receiver.blocking_recv().map_err(|_| Error::Todo)?;
        let cmds = cmds.map_err(|output| Error::WebhookFailure(output))?;
        for cmd in cmds {
            let req = cmd.lift(self.handle().clone());
            tracing::trace!("handling request {} from webhook", req);
            let _ = handle_request_aux(conn, &User::Admin, &req)?;
        }

        Ok(())
    }

    fn finish_refresh(
        &self,
        (url_locked, meta, actions_path): (String, ProjectMetadata, Option<String>),
    ) -> Result<TaskStatusKind, Error> {
        let mut conn = POOL.get().unwrap();
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
        decls: HashMap<String, typhon_types::requests::JobsetDecl>,
    ) -> Result<TaskStatusKind, Error> {
        let mut conn = POOL.get().unwrap();
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
