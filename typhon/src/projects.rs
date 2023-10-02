use crate::actions;
use crate::connection;
use crate::error::Error;
use crate::gcroots;
use crate::jobsets;
use crate::models;
use crate::nix;
use crate::schema;
use crate::CURRENT_SYSTEM;
use crate::{handles, responses};
use crate::{log_event, Event};

use typhon_types::responses::ProjectMetadata;

use age::secrecy::ExposeSecret;
use diesel::prelude::*;
use serde::Deserialize;

use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

pub struct Project {
    pub project: models::Project,
}

impl Project {
    pub async fn create(
        name: &String,
        decl: &typhon_types::requests::ProjectDecl,
    ) -> Result<(), Error> {
        let handle = handles::project(name.clone());
        if !handle.legal() {
            return Err(Error::IllegalProjectHandle(handle.clone()));
        }
        match Self::get(&handle).await {
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
                let mut conn = connection().await;
                diesel::insert_into(schema::projects::table)
                    .values(&new_project)
                    .execute(&mut *conn)?;
                drop(conn);
                log_event(Event::ProjectNew(handle.clone())).await;
                Ok(())
            }
        }
    }

    pub fn default_jobsets(&self) -> HashMap<String, jobsets::JobsetDecl> {
        HashMap::from([(
            "main".to_string(),
            jobsets::JobsetDecl {
                flake: true,
                url: self.project.url.clone(),
            },
        )])
    }

    pub async fn delete(&self) -> Result<(), Error> {
        let mut conn = connection().await;
        let jobsets: Vec<jobsets::Jobset> = schema::jobsets::table
            .load::<models::Jobset>(&mut *conn)?
            .drain(..)
            .map(|jobset| jobsets::Jobset {
                jobset,
                project: self.project.clone(),
            })
            .collect();
        drop(conn);

        for jobset in jobsets.iter() {
            jobset.delete().await?;
        }

        let mut conn = connection().await;
        diesel::delete(&self.project).execute(&mut *conn)?;
        drop(conn);

        log_event(Event::ProjectDeleted(self.handle())).await;

        Ok(())
    }

    pub async fn get(handle: &handles::Project) -> Result<Self, Error> {
        let mut conn = connection().await;
        let project = schema::projects::table
            .filter(schema::projects::name.eq(&handle.name))
            .first::<models::Project>(&mut *conn)
            .optional()?
            .ok_or(Error::ProjectNotFound(handle.clone()))?;
        Ok(Self { project })
    }

    pub fn handle(&self) -> handles::Project {
        handles::project(self.project.name.clone())
    }

    pub async fn info(&self) -> Result<responses::ProjectInfo, Error> {
        let mut conn = connection().await;
        let jobsets_names = schema::jobsets::table
            .filter(schema::jobsets::project_id.eq(&self.project.id))
            .load::<models::Jobset>(&mut *conn)?
            .iter()
            .map(|jobset| jobset.name.clone())
            .collect();
        drop(conn);
        let public_key = age::x25519::Identity::from_str(&self.project.key)
            .map_err(|_| Error::Todo)?
            .to_public()
            .to_string();
        Ok(responses::ProjectInfo {
            actions_path: self.project.actions_path.clone(),
            flake: self.project.flake,
            url: self.project.url.clone(),
            url_locked: self.project.url_locked.clone(),
            jobsets: jobsets_names,
            metadata: responses::ProjectMetadata {
                title: self.project.title.clone(),
                description: self.project.description.clone(),
                homepage: self.project.homepage.clone(),
            },
            public_key,
        })
    }

    pub async fn list() -> Result<Vec<(String, responses::ProjectMetadata)>, Error> {
        let mut conn = connection().await;
        Ok(schema::projects::table
            .order(schema::projects::name.asc())
            .load::<models::Project>(&mut *conn)?
            .iter()
            .map(|project| {
                (
                    project.name.clone(),
                    responses::ProjectMetadata {
                        title: project.title.clone(),
                        description: project.description.clone(),
                        homepage: project.homepage.clone(),
                    },
                )
            })
            .collect())
    }

    pub async fn refresh(&self) -> Result<(), Error> {
        let url_locked = nix::lock(&self.project.url).await?;

        #[derive(Deserialize)]
        struct TyphonProject {
            actions: Option<HashMap<String, String>>,
            #[serde(default)]
            metadata: ProjectMetadata,
        }

        let TyphonProject { actions, metadata } = serde_json::from_value(
            nix::eval(&url_locked, &"typhonProject", self.project.flake).await?,
        )
        .expect("TODO");

        let actions: Option<&String> = actions.as_ref().map(|m| m.get(&*CURRENT_SYSTEM)).flatten();

        let actions_path = if let Some(x) = actions {
            let drv = nix::derivation(nix::Expr::Path(x.clone())).await?;
            Some(nix::build(&drv.path).await?["out"].clone())
            // TODO: check public key used to encrypt secrets
        } else {
            None
        };

        let mut conn = connection().await;
        diesel::update(&self.project)
            .set((
                schema::projects::actions_path.eq(actions_path),
                schema::projects::description.eq(metadata.description),
                schema::projects::homepage.eq(metadata.homepage),
                schema::projects::title.eq(metadata.title),
                schema::projects::url_locked.eq(url_locked),
            ))
            .execute(&mut *conn)?;
        gcroots::update(&mut *conn);
        drop(conn);
        log_event(Event::ProjectUpdated(self.handle())).await;

        Ok(())
    }

    pub async fn set_decl(&self, decl: &typhon_types::requests::ProjectDecl) -> Result<(), Error> {
        let mut conn = connection().await;
        diesel::update(&self.project)
            .set((
                schema::projects::flake.eq(decl.flake),
                schema::projects::url.eq(&decl.url),
            ))
            .execute(&mut *conn)?;
        drop(conn);
        log_event(Event::ProjectUpdated(self.handle())).await;
        Ok(())
    }

    pub async fn set_private_key(&self, key: &String) -> Result<(), Error> {
        let _ = age::x25519::Identity::from_str(key).map_err(|_| Error::Todo)?;
        let mut conn = connection().await;
        diesel::update(&self.project)
            .set(schema::projects::key.eq(key))
            .execute(&mut *conn)?;
        drop(conn);
        log_event(Event::ProjectUpdated(self.handle())).await;
        Ok(())
    }

    pub async fn update_jobsets(&self) -> Result<Vec<String>, Error> {
        // run action `jobsets`
        let decls: HashMap<String, jobsets::JobsetDecl> = match &self.project.actions_path {
            Some(path) => {
                if Path::new(&format!("{}/jobsets", path)).exists() {
                    let action_input = serde_json::json!(null);
                    let (action_output, _) = actions::run(
                        &self.project.key,
                        &format!("{}/jobsets", path),
                        &format!("{}/secrets", path),
                        &action_input,
                    )
                    .await?;
                    serde_json::from_str(&action_output)
                        .map_err(|_| Error::BadJobsetDecl(action_output))?
                } else {
                    self.default_jobsets()
                }
            }
            None => self.default_jobsets(),
        };

        let mut conn = connection().await;
        let mut current_jobsets: Vec<jobsets::Jobset> = schema::jobsets::table
            .filter(schema::jobsets::project_id.eq(&self.project.id))
            .load::<models::Jobset>(&mut *conn)?
            .drain(..)
            .map(|jobset| jobsets::Jobset {
                project: self.project.clone(),
                jobset,
            })
            .collect();
        drop(conn);

        // delete obsolete jobsets
        let mut set = std::collections::HashSet::<String>::new();
        for jobset in current_jobsets.drain(..) {
            if decls
                .get(&jobset.jobset.name)
                .is_some_and(|decl| *decl == jobset.decl())
            {
                set.insert(jobset.jobset.name);
            } else {
                jobset.delete().await?;
            }
        }

        let mut conn = connection().await;

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
                    .execute(&mut *conn)?;
            }
        }

        gcroots::update(&mut *conn);

        drop(conn);

        log_event(Event::ProjectJobsetsUpdated(self.handle())).await;

        Ok(decls.into_keys().collect())
    }

    pub async fn webhook(
        &self,
        input: actions::webhooks::Input,
    ) -> Result<Vec<typhon_types::requests::Request>, Error> {
        match &self.project.actions_path {
            Some(path) => {
                if Path::new(&format!("{}/webhook", path)).exists() {
                    let action_input = serde_json::to_value(input).unwrap();
                    let (action_output, _) = actions::run(
                        &self.project.key,
                        &format!("{}/webhook", path),
                        &format!("{}/secrets", path),
                        &action_input,
                    )
                    .await?;
                    let commands: actions::webhooks::Output =
                        serde_json::from_str(&action_output).map_err(|_| Error::Todo)?;
                    Ok(commands
                        .into_iter()
                        .map(|cmd| cmd.lift(self.handle()))
                        .collect())
                } else {
                    Ok(Vec::new())
                }
            }
            None => Ok(Vec::new()),
        }
    }
}
