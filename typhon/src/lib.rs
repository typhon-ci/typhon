mod actions;
mod builds;
mod error;
mod evaluations;
mod events;
mod gcroots;
mod jobs;
mod jobsets;
mod models;
mod nix;
mod projects;
mod runs;
mod schema;
mod tasks;
mod time;

pub mod api;
pub mod build_manager;
pub mod logs;
pub mod task_manager;

pub use typhon_types::{
    handles, requests, responses, responses::Response, responses::ResponseError, Event,
};

use actions::Action;
use builds::Build;
use error::Error;
use evaluations::Evaluation;
use jobs::Job;
use jobsets::Jobset;
use projects::Project;
use runs::Run;
use task_manager::TaskManager;

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
pub static RUNS: Lazy<TaskManager<i32>> = Lazy::new(TaskManager::new);
pub static TASKS: Lazy<TaskManager<i32>> = Lazy::new(TaskManager::new);
pub static LOGS: Lazy<logs::live::Cache<i32>> = Lazy::new(logs::live::Cache::new);
pub static CONNECTION: Lazy<Connection> = Lazy::new(|| {
    use diesel::Connection as _;
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let conn = SqliteConnection::establish(&database_url)
        .unwrap_or_else(|e| panic!("Error connecting to {}, with error {:#?}", database_url, e));
    Connection::new(conn)
});
pub static EVENT_LOGGER: Lazy<events::EventLogger> = Lazy::new(events::EventLogger::new);
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
    use requests::*;
    match req {
        Request::ListEvaluations(_)
        | Request::ListProjects
        | Request::ListRuns(_)
        | Request::ListBuilds(_)
        | Request::ListActions(_)
        | Request::Project(_, Project::Info)
        | Request::Jobset(_, Jobset::Info)
        | Request::Evaluation(_, Evaluation::Info)
        | Request::Evaluation(_, Evaluation::Log)
        | Request::Job(_, Job::Info)
        | Request::Run(_, Run::Info)
        | Request::Build(_, Build::Info)
        | Request::Build(_, Build::Log)
        | Request::Action(_, Action::Info)
        | Request::Action(_, Action::Log)
        | Request::Login(_) => true,
        _ => user.is_admin(),
    }
}

pub async fn handle_request_aux(user: &User, req: &requests::Request) -> Result<Response, Error> {
    if authorize_request(user, req) {
        Ok(match req {
            requests::Request::ListEvaluations(search) => {
                Response::ListEvaluations(Evaluation::search(search).await?)
            }
            requests::Request::ListBuilds(search) => {
                Response::ListBuilds(Build::search(search).await?)
            }
            requests::Request::ListActions(search) => {
                Response::ListActions(Action::search(search).await?)
            }
            requests::Request::ListRuns(search) => Response::ListRuns(Run::search(search).await?),
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
                        project.update_jobsets().await?;
                        Response::Ok
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
                    requests::Jobset::Info => Response::JobsetInfo(jobset.info()),
                }
            }
            requests::Request::Evaluation(evaluation_handle, req) => {
                let evaluation = Evaluation::get(evaluation_handle).await?;
                match req {
                    requests::Evaluation::Cancel => {
                        evaluation.cancel().await;
                        Response::Ok
                    }
                    requests::Evaluation::Info => {
                        Response::EvaluationInfo(evaluation.info().await?)
                    }
                    requests::Evaluation::Log => Response::Log(evaluation.log().await?),
                }
            }
            requests::Request::Job(job_handle, req) => {
                let job = Job::get(&job_handle).await?;
                match req {
                    requests::Job::Info => Response::JobInfo(job.info()),
                }
            }
            requests::Request::Build(build_handle, req) => {
                let build = Build::get(&build_handle).await?;
                match req {
                    requests::Build::Info => Response::BuildInfo(build.info()),
                    requests::Build::Log => Response::Log(build.log().await?),
                }
            }
            requests::Request::Action(action_handle, req) => {
                let action = Action::get(&action_handle).await?;
                match req {
                    requests::Action::Info => Response::ActionInfo(action.info()),
                    requests::Action::Log => Response::Log(action.log().await?),
                }
            }
            requests::Request::Run(run_handle, req) => {
                let run = Run::get(&run_handle).await?;
                match req {
                    requests::Run::Cancel => {
                        run.cancel().await;
                        Response::Ok
                    }
                    requests::Run::Info => Response::RunInfo(run.info()),
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

pub async fn log_event(event: Event) {
    log::info!("event: {:?}", event);
    EVENT_LOGGER.log(event).await;
}

pub async fn shutdown() {
    eprintln!("Typhon is shutting down...");
    tokio::join!(
        RUNS.shutdown(),
        TASKS.shutdown(),
        LOGS.shutdown(),
        EVENT_LOGGER.shutdown(),
        build_manager::BUILDS.shutdown(),
    );
    eprintln!("Good bye!");
}
