mod actions;
mod builds;
mod error;
mod evaluations;
mod jobs;
mod jobsets;
mod models;
mod nix;
mod projects;
mod schema;
mod time;

pub mod api;
pub mod listeners;
pub mod tasks;

pub use typhon_types::{
    handles, requests, responses, responses::Response, responses::ResponseError, Event,
};

use error::Error;
use models::*;

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use diesel::prelude::*;
use log::*;
use once_cell::sync::OnceCell;
use sha256::digest;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct Settings {
    pub hashed_password: String,
}

pub struct Connection {
    pub conn: Mutex<SqliteConnection>,
}

impl Connection {
    pub fn new(conn: SqliteConnection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Sqlite Connection")
    }
}

// Typhon's state
pub static SETTINGS: OnceCell<Settings> = OnceCell::new();
pub static EVALUATIONS: OnceCell<tasks::Tasks<i32>> = OnceCell::new();
pub static BUILDS: OnceCell<tasks::Tasks<i32>> = OnceCell::new();
pub static JOBS: OnceCell<tasks::Tasks<i32>> = OnceCell::new();
pub static CONNECTION: OnceCell<Connection> = OnceCell::new();
pub static LISTENERS: OnceCell<Mutex<listeners::Listeners>> = OnceCell::new();

pub async fn connection<'a>() -> tokio::sync::MutexGuard<'a, SqliteConnection> {
    CONNECTION.get().unwrap().conn.lock().await
}

#[derive(Debug, Clone, Copy)]
pub enum User {
    Admin,
    Anonymous,
}

impl User {
    pub fn is_admin(&self) -> bool {
        match self {
            User::Admin => true,
            _ => false,
        }
    }
}

impl FromRequest for User {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<User, actix_web::Error>>>>;

    fn from_request(req: &HttpRequest, _pl: &mut Payload) -> Self::Future {
        let user = req
            .headers()
            .get("password")
            .map_or(User::Anonymous, |password| {
                let hash = digest(password.as_bytes());
                if hash == SETTINGS.get().unwrap().hashed_password {
                    User::Admin
                } else {
                    User::Anonymous
                }
            });
        Box::pin(async move { Ok(user) })
    }
}

pub fn authorize_request(user: &User, req: &requests::Request) -> bool {
    match req {
        requests::Request::ListProjects
        | requests::Request::Project(_, requests::Project::Info)
        | requests::Request::Jobset(_, requests::Jobset::Info)
        | requests::Request::Evaluation(_, requests::Evaluation::Info)
        | requests::Request::Job(_, requests::Job::Info)
        | requests::Request::Build(_, requests::Build::Info) => true,
        _ => user.is_admin(),
    }
}

pub async fn handle_request_aux(user: &User, req: &requests::Request) -> Result<Response, Error> {
    if authorize_request(user, req) {
        let conn = &mut *connection().await;
        Ok(match req {
            requests::Request::ListProjects => Response::ListProjects(Project::list(conn)?),
            requests::Request::CreateProject(project_handle) => {
                Project::create(conn, &project_handle)?;
                Response::Ok
            }
            requests::Request::Project(project_handle, req) => {
                let project = Project::get(conn, &project_handle)?;
                match req {
                    requests::Project::Delete => {
                        project.delete(conn)?;
                        Response::Ok
                    }
                    requests::Project::Info => Response::ProjectInfo(project.info(conn)?),
                    requests::Project::Refresh => {
                        project.refresh(conn).await?;
                        Response::Ok
                    }
                    requests::Project::SetDecl(flake) => {
                        project.set_decl(conn, &flake)?;
                        Response::Ok
                    }
                    requests::Project::SetPrivateKey(key) => {
                        project.set_private_key(conn, &key)?;
                        Response::Ok
                    }
                    requests::Project::UpdateJobsets => {
                        let jobsets = project.update_jobsets(conn).await?;
                        Response::ProjectUpdateJobsets(jobsets)
                    }
                }
            }
            requests::Request::Jobset(jobset_handle, req) => {
                let jobset = Jobset::get(conn, &jobset_handle)?;
                match req {
                    requests::Jobset::Evaluate => {
                        let evaluation_handle = jobset.evaluate(conn).await?;
                        Response::JobsetEvaluate(evaluation_handle)
                    }
                    requests::Jobset::Info => Response::JobsetInfo(jobset.info(conn)?),
                }
            }
            requests::Request::Evaluation(evaluation_handle, req) => {
                let evaluation = Evaluation::get(conn, evaluation_handle)?;
                match req {
                    requests::Evaluation::Cancel => {
                        evaluation.cancel(conn).await?;
                        Response::Ok
                    }
                    requests::Evaluation::Info => Response::EvaluationInfo(evaluation.info(conn)?),
                }
            }
            requests::Request::Job(job_handle, req) => {
                let job = Job::get(conn, &job_handle)?;
                match req {
                    requests::Job::Cancel => {
                        job.cancel(conn).await?;
                        Response::Ok
                    }
                    requests::Job::Info => Response::JobInfo(job.info(conn)?),
                }
            }
            requests::Request::Build(build_handle, req) => {
                let build = Build::get(conn, &build_handle)?;
                match req {
                    requests::Build::Cancel => {
                        build.cancel().await?;
                        Response::Ok
                    }
                    requests::Build::Info => Response::BuildInfo(build.info()?),
                    requests::Build::Log => Response::BuildLog, // build.log()?
                }
            }
        })
    } else {
        Err(Error::AccessDenied)
    }
}

/// Main entry point for Typhon requests
pub async fn handle_request(user: User, req: requests::Request) -> Result<Response, ResponseError> {
    info!("handling request {:?} for user {:?}", req, user);
    Ok(handle_request_aux(&user, &req).await.map_err(|e| {
        if e.is_internal() {
            error!(
                "request {:?} for user {:?} raised error: {:?}",
                req, user, e
            );
        }
        use {error::Error::*, ResponseError::*};
        match e {
            BuildNotFound(_)
            | EvaluationNotFound(_)
            | JobNotFound(_)
            | JobsetNotFound(_)
            | ProjectNotFound(_) => ResourceNotFound(format!("{}", e)),
            AccessDenied
            | ActionError(_)
            | BadJobsetDecl(_)
            | BuildNotRunning(_)
            | EvaluationNotRunning(_)
            | JobNotRunning(_)
            | NixError(_)
            | ProjectAlreadyExists(_) => BadRequest(format!("{}", e)),
            Todo | UnexpectedDatabaseError(_) => InternalError,
        }
    })?)
}

pub fn log_event(event: Event) {
    info!("event: {:?}", event);
    let _ = tokio::spawn(async move {
        let listeners = &*LISTENERS.get().unwrap();
        listeners.lock().await.log(event);
    });
}
