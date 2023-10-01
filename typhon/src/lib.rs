mod actions;
mod error;
mod evaluations;
mod gcroots;
mod jobs;
mod jobsets;
mod models;
mod nix;
mod projects;
mod schema;
mod time;

pub mod api;
pub mod listeners;
pub mod logs;
pub mod tasks;

pub use typhon_types::{
    handles, requests, responses, responses::Response, responses::ResponseError, Event,
};

use error::Error;
use models::*;

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use clap::Parser;
use diesel::prelude::*;
use once_cell::sync::Lazy;
use sha256::digest;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::Mutex;

/// Typhon, Nix-based continuous integration
#[derive(Parser, Debug)]
#[command(name = "Typhon")]
#[command(about = "Nix-based continuous integration", long_about = None)]
pub struct Args {
    /// Hashed password
    #[arg(long, short)]
    pub password: String,

    /// Webroot
    #[arg(long, short)]
    pub webroot: String,

    /// Silence all output
    #[arg(long, short)]
    pub quiet: bool,

    /// Verbose mode (-v, -vv, -vvv, etc)
    #[arg(long, short, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Timestamp (sec, ms, ns, none)
    #[arg(long, short)]
    pub ts: Option<stderrlog::Timestamp>,
}

#[derive(Debug)]
pub struct Settings {
    pub hashed_password: String,
    pub webroot: String,
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
pub static SETTINGS: Lazy<Settings> = Lazy::new(|| {
    let args = Args::parse();
    Settings {
        hashed_password: args.password.clone(),
        webroot: args.webroot.clone(),
    }
});
pub static EVALUATIONS: Lazy<tasks::Tasks<i32>> = Lazy::new(tasks::Tasks::new);
pub static JOBS_BEGIN: Lazy<tasks::Tasks<i32>> = Lazy::new(tasks::Tasks::new);
pub static JOBS_BUILD: Lazy<tasks::Tasks<i32>> = Lazy::new(tasks::Tasks::new);
pub static JOBS_END: Lazy<tasks::Tasks<i32>> = Lazy::new(tasks::Tasks::new);
pub static CONNECTION: Lazy<Connection> = Lazy::new(|| {
    use diesel::Connection as _;
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let conn = SqliteConnection::establish(&database_url)
        .unwrap_or_else(|e| panic!("Error connecting to {}, with error {:#?}", database_url, e));
    Connection::new(conn)
});
pub static LISTENERS: Lazy<Mutex<listeners::Listeners>> =
    Lazy::new(|| Mutex::new(listeners::Listeners::new()));
pub static BUILD_LOGS: Lazy<logs::live::Cache<nix::DrvPath>> = Lazy::new(logs::live::Cache::new);
pub static CURRENT_SYSTEM: Lazy<String> = Lazy::new(nix::current_system);

pub async fn connection<'a>() -> tokio::sync::MutexGuard<'a, SqliteConnection> {
    CONNECTION.conn.lock().await
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
            .get("token")
            .map_or(User::Anonymous, |password| {
                let hash = digest(password.as_bytes());
                if hash == SETTINGS.hashed_password {
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
        | requests::Request::Evaluation(_, requests::Evaluation::Log)
        | requests::Request::Job(_, requests::Job::Info)
        | requests::Request::Job(_, requests::Job::LogBegin)
        | requests::Request::Job(_, requests::Job::LogEnd)
        | requests::Request::Login(_) => true,
        _ => user.is_admin(),
    }
}

pub async fn handle_request_aux(user: &User, req: &requests::Request) -> Result<Response, Error> {
    if authorize_request(user, req) {
        Ok(match req {
            requests::Request::ListProjects => Response::ListProjects(Project::list().await?),
            requests::Request::CreateProject { name, decl } => {
                Project::create(name, decl).await?;
                Response::Ok
            }
            requests::Request::Project(project_handle, req) => {
                let project = Project::get(&project_handle).await?;
                match req {
                    requests::Project::Delete => {
                        project.delete().await?;
                        Response::Ok
                    }
                    requests::Project::Info => Response::ProjectInfo(project.info().await?),
                    requests::Project::Refresh => {
                        project.refresh().await?;
                        Response::Ok
                    }
                    requests::Project::SetDecl(decl) => {
                        project.set_decl(decl).await?;
                        Response::Ok
                    }
                    requests::Project::SetPrivateKey(key) => {
                        project.set_private_key(&key).await?;
                        Response::Ok
                    }
                    requests::Project::UpdateJobsets => {
                        let jobsets = project.update_jobsets().await?;
                        Response::ProjectUpdateJobsets(jobsets)
                    }
                }
            }
            requests::Request::Jobset(jobset_handle, req) => {
                let jobset = Jobset::get(&jobset_handle).await?;
                match req {
                    requests::Jobset::Evaluate(force) => {
                        let evaluation_handle = jobset.evaluate(*force).await?;
                        Response::JobsetEvaluate(evaluation_handle)
                    }
                    requests::Jobset::Info => Response::JobsetInfo(jobset.info().await?),
                }
            }
            requests::Request::Evaluation(evaluation_handle, req) => {
                let evaluation = Evaluation::get(evaluation_handle).await?;
                match req {
                    requests::Evaluation::Cancel => {
                        evaluation.cancel().await?;
                        Response::Ok
                    }
                    requests::Evaluation::Info => {
                        Response::EvaluationInfo(evaluation.info().await?)
                    }
                    requests::Evaluation::Log => {
                        let log = Log::get(handles::Log::Eval(evaluation_handle.clone())).await?;
                        Response::Log(log.log_stderr)
                    }
                }
            }
            requests::Request::Job(job_handle, req) => {
                let job = Job::get(&job_handle).await?;
                match req {
                    requests::Job::Cancel => {
                        job.cancel().await?;
                        Response::Ok
                    }
                    requests::Job::Info => Response::JobInfo(job.info()),
                    requests::Job::LogBegin => {
                        let log = Log::get(handles::Log::Begin(job_handle.clone())).await?;
                        Response::Log(log.log_stderr)
                    }
                    requests::Job::LogEnd => {
                        let log = Log::get(handles::Log::End(job_handle.clone())).await?;
                        Response::Log(log.log_stderr)
                    }
                }
            }
            requests::Request::Login(password) => {
                let hash = digest(password.as_bytes());
                if hash == SETTINGS.hashed_password {
                    Response::Login {
                        // TODO: manage session tokens instead of just returning the password
                        token: password.clone(),
                    }
                } else {
                    Err(Error::LoginError)?
                }
            }
        })
    } else {
        Err(Error::AccessDenied)
    }
}

/// Main entry point for Typhon requests
pub async fn handle_request(user: User, req: requests::Request) -> Result<Response, ResponseError> {
    log::info!("handling request {} for user {:?}", req, user);
    Ok(handle_request_aux(&user, &req).await.map_err(|e| {
        if e.is_internal() {
            log::error!(
                "request {:?} for user {:?} raised error: {:?}",
                req,
                user,
                e
            );
        }
        e.into()
    })?)
}

pub fn log_event(event: Event) {
    log::info!("event: {:?}", event);
    let _ = tokio::spawn(async move {
        LISTENERS.lock().await.log(event);
    });
}

pub async fn shutdown() {
    eprintln!("Typhon is shutting down...");
    tokio::join!(
        EVALUATIONS.shutdown(),
        JOBS_BUILD.shutdown(),
        JOBS_BEGIN.shutdown(),
        JOBS_END.shutdown(),
    );
    eprintln!("Good bye!");
}
