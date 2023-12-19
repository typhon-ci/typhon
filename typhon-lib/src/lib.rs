#![feature(impl_trait_in_fn_trait_return)]

mod actions;
mod builds;
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
mod search;
mod tasks;

pub mod build_manager;
pub mod error;
pub mod logs;
pub mod task_manager;
use search::search;

pub use typhon_types::data;
pub use typhon_types::{
    handles, requests, responses, responses::Response, responses::ResponseError, Event,
};

pub use crate::actions::webhooks;

use actions::Action;
use builds::Build;
use error::Error;
use evaluations::Evaluation;
use jobs::Job;
use jobsets::Jobset;
use projects::Project;
use runs::Run;
use task_manager::TaskManager;

use diesel::prelude::*;
use diesel::r2d2;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sha256::digest;

#[derive(Debug)]
pub struct Settings {
    pub hashed_password: String,
}

pub type DbPool = r2d2::Pool<r2d2::ConnectionManager<diesel::SqliteConnection>>;
pub type Conn =
    diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::SqliteConnection>>;

#[derive(Debug)]
pub struct ConnectionCustomizer {}

impl diesel::r2d2::CustomizeConnection<diesel::SqliteConnection, diesel::r2d2::Error>
    for ConnectionCustomizer
{
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        use diesel::connection::SimpleConnection;
        (|| {
            conn.batch_execute("PRAGMA foreign_keys = ON;")?;
            conn.batch_execute("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
            conn.batch_execute("PRAGMA busy_timeout = 10000;")?;
            Ok(())
        })()
        .map_err(diesel::r2d2::Error::QueryError)
    }
}

// Typhon's state
pub static RUNTIME: Lazy<tokio::runtime::Runtime> =
    Lazy::new(|| tokio::runtime::Runtime::new().unwrap());
pub static POOL: Lazy<DbPool> = Lazy::new(pool);
pub static SETTINGS: Lazy<Settings> = Lazy::new(|| Settings {
    hashed_password: std::env::var("HASHED_PASSWORD").unwrap(),
});
pub static RUNS: Lazy<TaskManager<i32>> = Lazy::new(|| TaskManager::new());
pub static TASKS: Lazy<TaskManager<i32>> = Lazy::new(|| TaskManager::new());
pub static LOGS: Lazy<logs::live::Cache<i32>> = Lazy::new(logs::live::Cache::new);
pub static EVENT_LOGGER: Lazy<events::EventLogger> = Lazy::new(events::EventLogger::new);
pub static CURRENT_SYSTEM: Lazy<String> = Lazy::new(nix::current_system);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
    pub fn from_password(password: &[u8]) -> Self {
        let hash = digest(password);
        if hash == SETTINGS.hashed_password {
            User::Admin
        } else {
            User::Anonymous
        }
    }
}

pub fn authorize_request(user: &User, req: &requests::Request) -> bool {
    use requests::*;
    match req {
        Request::Search { .. }
        | Request::Project(_, Project::Info)
        | Request::Jobset(_, Jobset::Info)
        | Request::Evaluation(_, Evaluation::Info | Evaluation::Log)
        | Request::Job(_, Job::Info)
        | Request::Run(_, Run::Info)
        | Request::Build(_, Build::Info | Build::Log)
        | Request::Action(_, Action::Info | Action::Log)
        | Request::Login { .. }
        | Request::User => true,
        _ => user.is_admin(),
    }
}

pub fn handle_request_aux(
    conn: &mut Conn,
    user: &User,
    req: &requests::Request,
) -> Result<Response, Error> {
    if !authorize_request(user, req) {
        return Err(Error::AccessDenied);
    }
    Ok(match req {
        requests::Request::Search(requests::search::Request {
            limit,
            offset,
            kind,
        }) => search(*limit, *offset, kind, conn)?,
        requests::Request::CreateProject { name, decl } => {
            Project::create(conn, name, decl)?;
            Response::Ok
        }
        requests::Request::Project(project_handle, req) => {
            let project = Project::get(conn, &project_handle)?;
            match req {
                requests::Project::Info => return Ok(Response::ProjectInfo(project.info(conn)?)),
                requests::Project::Refresh => project.refresh(conn)?,
                requests::Project::SetDecl(decl) => project.set_decl(conn, decl)?,
                requests::Project::SetPrivateKey(key) => project.set_private_key(conn, &key)?,
                requests::Project::UpdateJobsets => project.update_jobsets(conn)?,
            };
            Response::Ok
        }
        requests::Request::Jobset(jobset_handle, req) => {
            let jobset = Jobset::get(conn, &jobset_handle)?;
            match req {
                requests::Jobset::Evaluate(force) => {
                    let evaluation_handle = jobset.evaluate(conn, *force)?;
                    Response::JobsetEvaluate(evaluation_handle)
                }
                requests::Jobset::Info => Response::JobsetInfo(jobset.info()),
            }
        }
        requests::Request::Evaluation(evaluation_handle, req) => {
            let evaluation = Evaluation::get(conn, evaluation_handle)?;
            match req {
                requests::Evaluation::Cancel => {
                    evaluation.cancel();
                    Response::Ok
                }
                requests::Evaluation::Info => Response::EvaluationInfo(evaluation.info(conn)?),
                requests::Evaluation::Log => Response::Log(evaluation.log(conn)?),
            }
        }
        requests::Request::Job(job_handle, req) => {
            let job = Job::get(conn, &job_handle)?;
            match req {
                requests::Job::Info => Response::JobInfo(job.info(conn)?),
                requests::Job::Rerun => {
                    job.rerun(conn)?;
                    Response::Ok
                }
            }
        }
        requests::Request::Build(build_handle, req) => {
            let build = Build::get(conn, &build_handle)?;
            match req {
                requests::Build::Info => Response::BuildInfo(build.info()),
                requests::Build::Log => Response::Log(build.log(conn)?),
            }
        }
        requests::Request::Action(action_handle, req) => {
            let action = Action::get(conn, &action_handle)?;
            match req {
                requests::Action::Info => Response::ActionInfo(action.info()),
                requests::Action::Log => Response::Log(action.log(conn)?),
            }
        }
        requests::Request::Run(run_handle, req) => {
            let run = Run::get(conn, &run_handle)?;
            match req {
                //requests::Run::Cancel => {
                //    run.cancel();
                //    Response::Ok
                //}
                requests::Run::Info => Response::RunInfo(run.info()),
            }
        }
        requests::Request::Login { password } => {
            let hash = digest(password.as_bytes());
            if hash == SETTINGS.hashed_password {
                Response::Ok
            } else {
                Err(Error::LoginError)?
            }
        }
        requests::Request::User => Response::User(match user {
            User::Admin => Some(data::User::Admin),
            User::Anonymous => None,
        }),
    })
}

/// Main entry point for Typhon requests
pub async fn handle_request(user: User, req: requests::Request) -> Result<Response, ResponseError> {
    RUNTIME
        .spawn_blocking(move || {
            let mut conn = POOL.get().unwrap();
            log::trace!("handling request {} for user {:?}", req, user);
            handle_request_aux(&mut conn, &user, &req).map_err(|e| {
                if e.is_internal() {
                    log::error!("request {} for user {:?} raised error: {:?}", req, user, e,);
                } else {
                    log::debug!("request {} for user {:?} raised error: {:?}", req, user, e,);
                }
                e.into()
            })
        })
        .await
        .unwrap()
}

pub fn log_event(event: Event) {
    log::trace!("event: {:?}", event);
    EVENT_LOGGER.log(event);
}

pub fn live_log_build(
    handle: handles::Build,
) -> Result<Option<impl futures_core::stream::Stream<Item = String>>, Error> {
    let mut conn = POOL.get().unwrap();
    let build = builds::Build::get(&mut conn, &handle)?;
    Ok(LOGS.listen(&build.task.task.id))
}

pub fn live_log_action(
    handle: handles::Action,
) -> Result<Option<impl futures_core::stream::Stream<Item = String>>, Error> {
    let mut conn = POOL.get().unwrap();
    let action = actions::Action::get(&mut conn, &handle)?;
    Ok(LOGS.listen(&action.task.task.id))
}

pub fn webhook(
    project_handle: handles::Project,
    input: actions::webhooks::Input,
) -> Result<Vec<requests::Request>, Error> {
    let mut conn = POOL.get().unwrap();

    log::debug!("handling webhook {:?}", input);

    let project = projects::Project::get(&mut conn, &project_handle).map_err(|e| {
        if e.is_internal() {
            log::error!("webhook raised error: {:?}", e);
        }
        e
    })?;

    let res = project.webhook(&mut conn, input).map_err(|e| {
        if e.is_internal() {
            log::error!("webhook raised error: {:?}", e);
        }
        e
    })?;

    if res.is_none() {
        log::warn!("bad webhook for project {}", project_handle);
    }

    match res {
        Some(requests) => Ok(requests),
        None => Err(error::Error::BadWebhookOutput)?,
    }
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

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

fn pool() -> DbPool {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = diesel::r2d2::ConnectionManager::<SqliteConnection>::new(database_url);
    let pool = diesel::r2d2::Pool::builder()
        .connection_customizer(Box::new(ConnectionCustomizer {}))
        .build(manager)
        .expect("database URL should be valid path to SQLite DB file");

    // Run migrations
    let mut conn = pool.get().unwrap();
    let _ = conn
        .run_pending_migrations(MIGRATIONS)
        .expect("failed to run migrations");

    pool
}
