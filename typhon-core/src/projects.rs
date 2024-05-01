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

    //pub fn delete(&self, _conn: &mut Conn) -> Result<(), Error> {
    //    todo!()
    //}

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

    pub fn info(
        conn: &mut Conn,
        handle: handles::Project,
    ) -> Result<responses::ProjectInfo, Error> {
        let (project, refresh_task): (models::Project, Option<models::Task>) =
            schema::projects::table
                .left_join(schema::tasks::table)
                .filter(schema::projects::name.eq(&handle.name))
                .first(conn)
                .optional()?
                .ok_or(Error::ProjectNotFound(handle.clone()))?;
        let jobsets = schema::jobsets::table
            .filter(schema::jobsets::project_id.eq(&project.id))
            .load::<models::Jobset>(conn)?
            .iter()
            .map(|jobset| jobset.name.clone())
            .collect();
        let public_key = age::x25519::Identity::from_str(&project.key)
            .map_err(|_| Error::Todo)?
            .to_public()
            .to_string();
        let models::Project {
            actions_path,
            description,
            flake,
            homepage,
            title,
            url,
            url_locked,
            ..
        } = project;
        let last_refresh = refresh_task.map(|task| task.status());
        Ok(responses::ProjectInfo {
            actions_path,
            flake,
            handle,
            jobsets,
            last_refresh,
            metadata: ProjectMetadata {
                title,
                description,
                homepage,
            },
            public_key,
            url,
            url_locked,
        })
    }

    pub fn new_action(
        &self,
        conn: &mut Conn,
        path: &String,
        name: &String,
        input: &serde_json::Value,
    ) -> Result<models::ActionPlus, Error> {
        use uuid::{timestamp, Uuid};

        conn.transaction::<models::ActionPlus, Error, _>(|conn| {
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
            Ok(models::ActionPlus {
                action,
                key: self.project.key.clone(),
                project_name: self.project.name.clone(),
            })
        })
    }

    pub fn refresh(conn: &mut Conn, handle: handles::Project) -> Result<(), Error> {
        #[derive(Deserialize)]
        struct TyphonProject {
            actions: Option<HashMap<String, String>>,
            #[serde(default)]
            meta: ProjectMetadata,
        }

        let project: models::Project = schema::projects::table
            .filter(schema::projects::name.eq(&handle.name))
            .first(conn)
            .optional()?
            .ok_or(Error::ProjectNotFound(handle.clone()))?;

        let models::Project { flake, id, url, .. } = project;

        let run = move |sender| async move {
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
        };

        let finish = {
            let handle = handle.clone();
            move |res: Option<Result<(String, ProjectMetadata, Option<String>), Error>>| {
                let status = match res {
                    Some(Ok(x)) => Project::finish_refresh(id, x),
                    Some(Err(e)) => {
                        tracing::warn!("refresh error for project {}: {}", handle, e);
                        Ok(TaskStatusKind::Failure)
                    }
                    None => Ok(TaskStatusKind::Canceled),
                };
                (
                    status.unwrap_or(TaskStatusKind::Failure),
                    Event::ProjectUpdated(handle),
                )
            }
        };

        let task = tasks::Task::new(conn)?;
        diesel::update(schema::projects::table)
            .filter(schema::projects::id.eq(id))
            .set(schema::projects::last_refresh_task_id.eq(task.task.id))
            .execute(conn)?;

        log_event(Event::ProjectUpdated(handle));

        task.run(conn, run, finish)?;

        Ok(())
    }

    pub fn set_decl(
        conn: &mut Conn,
        handle: handles::Project,
        decl: &typhon_types::requests::ProjectDecl,
    ) -> Result<(), Error> {
        diesel::update(schema::projects::table)
            .filter(schema::projects::name.eq(&handle.name))
            .set((
                schema::projects::flake.eq(decl.flake),
                schema::projects::url.eq(&decl.url),
            ))
            .execute(conn)
            .or(Err(Error::ProjectNotFound(handle.clone())))?;
        log_event(Event::ProjectUpdated(handle));
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
                        let decls: Result<HashMap<String, jobsets::JobsetDecl>, Error> =
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
            move |output: Option<String>| match output {
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
                        TaskStatusKind::Failure
                    }
                },
                None => {
                    let _ = sender.send(None);
                    TaskStatusKind::Failure
                }
            }
        };

        action.spawn(conn, finish)?;

        Ok(receiver.blocking_recv().map_err(|_| Error::Todo)?)
    }

    fn finish_refresh(
        id: i32,
        (url_locked, meta, actions_path): (String, ProjectMetadata, Option<String>),
    ) -> Result<TaskStatusKind, Error> {
        let mut conn = POOL.get().unwrap();
        diesel::update(schema::projects::table)
            .filter(schema::projects::id.eq(id))
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
