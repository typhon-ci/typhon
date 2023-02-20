use crate::actions;
use crate::connection;
use crate::error::Error;
use crate::jobsets::JobsetDecl;
use crate::models::*;
use crate::nix;
use crate::schema::jobsets::dsl::*;
use crate::schema::projects::dsl::*;
use crate::{handles, responses};
use crate::{log_event, Event};
use age::secrecy::ExposeSecret;
use diesel::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

impl Project {
    pub async fn create(project_handle: &handles::Project) -> Result<(), Error> {
        match Self::get(project_handle).await {
            Ok(_) => Err(Error::ProjectAlreadyExists(project_handle.clone())),
            Err(_) => {
                let key = age::x25519::Identity::generate()
                    .to_string()
                    .expose_secret()
                    .clone();
                let new_project = NewProject {
                    project_name: &project_handle.project,
                    project_key: &key,
                };
                let mut conn = connection().await;
                diesel::insert_into(projects)
                    .values(&new_project)
                    .execute(&mut *conn)?;
                drop(conn);
                log_event(Event::ProjectNew(project_handle.clone()));
                Ok(())
            }
        }
    }

    pub fn default_jobsets(&self) -> HashMap<String, JobsetDecl> {
        HashMap::from([(
            "main".to_string(),
            JobsetDecl {
                flake: self.project_decl.clone(),
            },
        )])
    }

    pub async fn delete(&self) -> Result<(), Error> {
        let mut conn = connection().await;
        diesel::delete(projects.find(self.project_id)).execute(&mut *conn)?;
        log_event(Event::ProjectDeleted(self.handle()));
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
                    project: project_name_.clone(),
                })
            })?)
    }

    pub fn handle(&self) -> handles::Project {
        handles::Project {
            project: self.project_name.clone(),
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
            metadata: responses::ProjectMetadata {
                title: self.project_title.clone(),
                description: self.project_description.clone(),
                homepage: self.project_homepage.clone(),
            },
            jobsets: jobsets_names,
            public_key: public_key,
            decl: self.project_decl.clone(),
            decl_locked: self.project_decl_locked.clone(),
            actions_path: self.project_actions_path.clone(),
        })
    }

    pub async fn list() -> Result<Vec<String>, Error> {
        let mut conn = connection().await;
        Ok(projects
            .order(project_name.asc())
            .load::<Project>(&mut *conn)?
            .iter()
            .map(|project| project.project_name.clone())
            .collect())
    }

    pub async fn refresh(&self) -> Result<(), Error> {
        let locked_flake = nix::lock(&self.project_decl).await?;
        let mut title = String::new();
        let mut description = String::new();
        let mut homepage = String::new();
        let mut actions_path = String::new();

        let expr = format!("{}#typhonProject", locked_flake);
        let typhon_project = nix::eval(expr).await?;

        typhon_project.get("meta").map(|metadata| {
            metadata
                .get("title")
                .map(|v| v.as_str().map(|s| title = s.to_string()));
            metadata
                .get("description")
                .map(|v| v.as_str().map(|s| description = s.to_string()));
            metadata
                .get("homepage")
                .map(|v| v.as_str().map(|s| homepage = s.to_string()));
        });

        match typhon_project.get("actions") {
            Some(v) => {
                let drv = nix::derivation_path(v.as_str().ok_or(Error::Todo)?.to_string()).await?;
                actions_path = nix::build(drv).await?;
                // TODO: check public key used to encrypt secrets
                Ok(())
            }
            None => Ok::<(), Error>(()),
        }?;

        let mut conn = connection().await;
        diesel::update(projects.find(self.project_id))
            .set((
                project_title.eq(title),
                project_description.eq(description),
                project_homepage.eq(homepage),
                project_actions_path.eq(actions_path),
                project_decl_locked.eq(locked_flake),
            ))
            .execute(&mut *conn)?;
        drop(conn);
        log_event(Event::ProjectUpdated(self.handle()));

        Ok(())
    }

    pub async fn set_decl(&self, flake: &String) -> Result<(), Error> {
        let mut conn = connection().await;
        diesel::update(projects.find(self.project_id))
            .set(project_decl.eq(flake))
            .execute(&mut *conn)?;
        drop(conn);
        log_event(Event::ProjectUpdated(self.handle()));
        Ok(())
    }

    pub async fn set_private_key(&self, key: &String) -> Result<(), Error> {
        let _ = age::x25519::Identity::from_str(key).map_err(|_| Error::Todo)?;
        let mut conn = connection().await;
        diesel::update(projects.find(self.project_id))
            .set(project_key.eq(key))
            .execute(&mut *conn)?;
        drop(conn);
        log_event(Event::ProjectUpdated(self.handle()));
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
                            .set(jobset_flake.eq(decl.flake.clone()))
                            .execute(conn)?;
                        Ok::<(), Error>(())
                    })
                    .unwrap_or_else(|| {
                        let new_jobset = NewJobset {
                            jobset_project: self.project_id,
                            jobset_name: name,
                            jobset_flake: &decl.flake,
                        };
                        diesel::insert_into(jobsets)
                            .values(&new_jobset)
                            .execute(conn)?;
                        Ok::<(), Error>(())
                    })?;
            }

            Ok(())
        })?;
        drop(conn);

        log_event(Event::ProjectJobsetsUpdated(self.handle()));

        Ok(decls.into_keys().collect())
    }
}
