use crate::actions;
use crate::connection;
use crate::error::Error;
use crate::gcroots;
use crate::jobsets::JobsetDecl;
use crate::models::*;
use crate::nix;
use crate::schema::jobsets::dsl::*;
use crate::schema::projects::dsl::*;
use crate::CURRENT_SYSTEM;
use crate::{handles, responses};
use crate::{log_event, Event};

use age::secrecy::ExposeSecret;
use diesel::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use typhon_types::responses::ProjectMetadata;

impl Project {
    pub async fn create(
        name: &String,
        decl: &typhon_types::requests::ProjectDecl,
    ) -> Result<(), Error> {
        let project_handle = handles::Project {
            name: name.to_string(),
        };
        if !project_handle.legal() {
            return Err(Error::IllegalProjectHandle(project_handle.clone()));
        }
        match Self::get(&project_handle).await {
            Ok(_) => Err(Error::ProjectAlreadyExists(project_handle.clone())),
            Err(_) => {
                let key = age::x25519::Identity::generate()
                    .to_string()
                    .expose_secret()
                    .clone();
                let new_project = NewProject {
                    project_url: &decl.url,
                    project_key: &key,
                    project_legacy: decl.legacy,
                    project_name: &project_handle.name,
                };
                let mut conn = connection().await;
                diesel::insert_into(projects)
                    .values(&new_project)
                    .execute(&mut *conn)?;
                drop(conn);
                log_event(Event::ProjectNew(project_handle.clone())).await;
                Ok(())
            }
        }
    }

    pub fn default_jobsets(&self) -> HashMap<String, JobsetDecl> {
        HashMap::from([(
            "main".to_string(),
            JobsetDecl {
                url: self.project_url.clone(),
                legacy: false,
            },
        )])
    }

    pub async fn delete(&self) -> Result<(), Error> {
        let mut conn = connection().await;
        diesel::delete(projects.find(self.project_id)).execute(&mut *conn)?;
        log_event(Event::ProjectDeleted(self.handle())).await;
        Ok(())
    }

    pub async fn get(project_handle: &handles::Project) -> Result<Self, Error> {
        let handles::pattern!(project_name_) = project_handle;
        let mut conn = connection().await;
        Ok(projects
            .filter(project_name.eq(project_name_))
            .first::<Project>(&mut *conn)
            .map_err(|_| {
                Error::ProjectNotFound(handles::Project {
                    name: project_name_.clone(),
                })
            })?)
    }

    pub fn handle(&self) -> handles::Project {
        handles::Project {
            name: self.project_name.clone(),
        }
    }

    pub async fn info(&self) -> Result<responses::ProjectInfo, Error> {
        let mut conn = connection().await;
        let jobsets_names = jobsets
            .filter(jobset_project.eq(self.project_id))
            .load::<Jobset>(&mut *conn)?
            .iter()
            .map(|jobset| jobset.jobset_name.clone())
            .collect();
        drop(conn);
        let public_key = age::x25519::Identity::from_str(&self.project_key)
            .map_err(|_| Error::Todo)?
            .to_public()
            .to_string();
        Ok(responses::ProjectInfo {
            actions_path: self.project_actions_path.clone(),
            url: self.project_url.clone(),
            url_locked: self.project_url_locked.clone(),
            jobsets: jobsets_names,
            legacy: self.project_legacy,
            metadata: responses::ProjectMetadata {
                title: self.project_title.clone(),
                description: self.project_description.clone(),
                homepage: self.project_homepage.clone(),
            },
            public_key,
        })
    }

    pub async fn list() -> Result<Vec<(String, responses::ProjectMetadata)>, Error> {
        let mut conn = connection().await;
        Ok(projects
            .order(project_name.asc())
            .load::<Project>(&mut *conn)?
            .iter()
            .map(|project| {
                (
                    project.project_name.clone(),
                    responses::ProjectMetadata {
                        title: project.project_title.clone(),
                        description: project.project_description.clone(),
                        homepage: project.project_homepage.clone(),
                    },
                )
            })
            .collect())
    }

    pub async fn refresh(&self) -> Result<(), Error> {
        let url_locked = nix::lock(&self.project_url).await?;

        #[derive(Deserialize)]
        struct TyphonProject {
            actions: Option<HashMap<String, String>>,
            #[serde(default)]
            metadata: ProjectMetadata,
        }

        let TyphonProject { actions, metadata } = serde_json::from_value(
            nix::eval(&url_locked, &"typhonProject", self.project_legacy).await?,
        )
        .expect("TODO");

        let actions: Option<&String> = actions.as_ref().map(|m| m.get(&*CURRENT_SYSTEM)).flatten();

        let actions_path = if let Some(v) = actions {
            let drv = nix::derivation(nix::Expr::Path(v.clone())).await?;
            Some(nix::build(&drv.path).await?["out"].clone())
            // TODO: check public key used to encrypt secrets
        } else {
            None
        };

        let mut conn = connection().await;
        diesel::update(projects.find(self.project_id))
            .set((
                project_title.eq(metadata.title),
                project_description.eq(metadata.description),
                project_homepage.eq(metadata.homepage),
                project_actions_path.eq(actions_path),
                project_url_locked.eq(url_locked),
            ))
            .execute(&mut *conn)?;
        gcroots::update(&mut *conn);
        drop(conn);
        log_event(Event::ProjectUpdated(self.handle())).await;

        Ok(())
    }

    pub async fn set_decl(&self, decl: &typhon_types::requests::ProjectDecl) -> Result<(), Error> {
        let mut conn = connection().await;
        diesel::update(projects.find(self.project_id))
            .set((project_url.eq(&decl.url), project_legacy.eq(decl.legacy)))
            .execute(&mut *conn)?;
        drop(conn);
        log_event(Event::ProjectUpdated(self.handle())).await;
        Ok(())
    }

    pub async fn set_private_key(&self, key: &String) -> Result<(), Error> {
        let _ = age::x25519::Identity::from_str(key).map_err(|_| Error::Todo)?;
        let mut conn = connection().await;
        diesel::update(projects.find(self.project_id))
            .set(project_key.eq(key))
            .execute(&mut *conn)?;
        drop(conn);
        log_event(Event::ProjectUpdated(self.handle())).await;
        Ok(())
    }

    pub async fn update_jobsets(&self) -> Result<Vec<String>, Error> {
        // run action `jobsets`
        let decls: HashMap<String, JobsetDecl> = match &self.project_actions_path {
            Some(path) => {
                if Path::new(&format!("{}/jobsets", path)).exists() {
                    let action_input = serde_json::json!(null);
                    let (action_output, _) = actions::run(
                        &self.project_key,
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
        conn.transaction::<(), Error, _>(|conn| {
            let current_jobsets = jobsets
                .filter(jobset_project.eq(self.project_id))
                .load::<Jobset>(conn)?;

            // delete obsolete jobsets
            for jobset in &current_jobsets {
                if !decls.contains_key(&jobset.jobset_name) {
                    diesel::delete(jobsets.find(jobset.jobset_id)).execute(conn)?;
                }
            }

            // create new jobsets or update old ones
            for (name, decl) in decls.iter() {
                current_jobsets
                    .iter()
                    .find(|&jobset| jobset.jobset_name == *name)
                    .map(|jobset| {
                        diesel::update(jobsets.find(jobset.jobset_id))
                            .set(jobset_url.eq(decl.url.clone()))
                            .execute(conn)?;
                        Ok::<(), Error>(())
                    })
                    .unwrap_or_else(|| {
                        let new_jobset = NewJobset {
                            jobset_project: self.project_id,
                            jobset_name: name,
                            jobset_url: &decl.url,
                            jobset_legacy: decl.legacy,
                        };
                        diesel::insert_into(jobsets)
                            .values(&new_jobset)
                            .execute(conn)?;
                        Ok::<(), Error>(())
                    })?;
            }

            Ok(())
        })?;
        gcroots::update(&mut *conn);
        drop(conn);

        log_event(Event::ProjectJobsetsUpdated(self.handle())).await;

        Ok(decls.into_keys().collect())
    }

    pub async fn webhook(
        &self,
        input: actions::webhooks::Input,
    ) -> Result<Vec<typhon_types::requests::Request>, Error> {
        match &self.project_actions_path {
            Some(path) => {
                if Path::new(&format!("{}/webhook", path)).exists() {
                    let action_input = serde_json::to_value(input).unwrap();
                    let (action_output, _) = actions::run(
                        &self.project_key,
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
