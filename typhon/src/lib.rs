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

use clap::Parser;
use diesel::prelude::*;
use diesel::r2d2;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use once_cell::sync::Lazy;
use sha256::digest;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

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
pub static SENDER: Lazy<mpsc::Sender<Msg>> = Lazy::new(init);
pub static RUNTIME: Lazy<tokio::runtime::Runtime> =
    Lazy::new(|| tokio::runtime::Runtime::new().unwrap());
pub static POOL: Lazy<DbPool> = Lazy::new(|| {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = diesel::r2d2::ConnectionManager::<SqliteConnection>::new(database_url);
    diesel::r2d2::Pool::builder()
        .connection_customizer(Box::new(ConnectionCustomizer {}))
        .build(manager)
        .expect("database URL should be valid path to SQLite DB file")
});
pub static SETTINGS: Lazy<Settings> = Lazy::new(|| {
    let args = Args::parse();
    Settings {
        hashed_password: args.password.clone(),
        webroot: args.webroot.clone(),
    }
});
pub static RUNS: Lazy<TaskManager<i32, DbPool>> = Lazy::new(|| TaskManager::new(&POOL));
pub static TASKS: Lazy<TaskManager<i32, DbPool>> = Lazy::new(|| TaskManager::new(&POOL));
pub static LOGS: Lazy<logs::live::Cache<i32>> = Lazy::new(logs::live::Cache::new);
pub static EVENT_LOGGER: Lazy<events::EventLogger> = Lazy::new(events::EventLogger::new);
pub static CURRENT_SYSTEM: Lazy<String> = Lazy::new(nix::current_system);

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
    pub fn from_token(token: Option<&[u8]>) -> Self {
        match token {
            Some(password) => {
                let hash = digest(password);
                if hash == SETTINGS.hashed_password {
                    User::Admin
                } else {
                    User::Anonymous
                }
            }
            None => User::Anonymous,
        }
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

pub fn handle_request_aux(
    conn: &mut Conn,
    user: &User,
    req: &requests::Request,
) -> Result<Response, Error> {
    if authorize_request(user, req) {
        Ok(match req {
            requests::Request::ListEvaluations(search) => {
                Response::ListEvaluations(Evaluation::search(conn, search)?)
            }
            requests::Request::ListBuilds(search) => {
                Response::ListBuilds(Build::search(conn, search)?)
            }
            requests::Request::ListActions(search) => {
                Response::ListActions(Action::search(conn, search)?)
            }
            requests::Request::ListRuns(search) => Response::ListRuns(Run::search(conn, search)?),
            requests::Request::ListProjects => Response::ListProjects(Project::list(conn)?),
            requests::Request::CreateProject { name, decl } => {
                Project::create(conn, name, decl)?;
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
                        project.refresh(conn)?;
                        Response::Ok
                    }
                    requests::Project::SetDecl(decl) => {
                        project.set_decl(conn, decl)?;
                        Response::Ok
                    }
                    requests::Project::SetPrivateKey(key) => {
                        project.set_private_key(conn, &key)?;
                        Response::Ok
                    }
                    requests::Project::UpdateJobsets => {
                        project.update_jobsets(conn)?;
                        Response::Ok
                    }
                }
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
                    requests::Job::Info => Response::JobInfo(job.info()),
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
                    requests::Run::Cancel => {
                        run.cancel();
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
    let (send, recv) = oneshot::channel();
    let _ = SENDER.send(Msg { send, user, req }).await;
    recv.await.unwrap()
}

pub fn log_event(event: Event) {
    log::info!("event: {:?}", event);
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

    log::info!("handling webhook {:?}", input);

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

pub struct Msg {
    pub user: User,
    pub req: requests::Request,
    pub send: oneshot::Sender<Result<Response, ResponseError>>,
}

pub async fn handler(mut recv: mpsc::Receiver<Msg>) {
    use tokio::task::spawn_blocking;

    while let Some(msg) = recv.recv().await {
        spawn_blocking(move || {
            let mut conn = POOL.get().unwrap();
            log::info!("handling request {} for user {:?}", msg.req, msg.user);
            let rsp = handle_request_aux(&mut conn, &msg.user, &msg.req).map_err(|e| {
                if e.is_internal() {
                    log::error!(
                        "request {:?} for user {:?} raised error: {:?}",
                        msg.req,
                        msg.user,
                        e,
                    );
                } else {
                    log::info!(
                        "request {:?} for user {:?} raised error: {:?}",
                        msg.req,
                        msg.user,
                        e,
                    );
                }
                e.into()
            });
            let _ = msg.send.send(rsp);
        });
    }
}

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

fn init() -> mpsc::Sender<Msg> {
    // Connect to the sqlite database
    let pool = POOL.clone();
    let mut conn = pool.get().unwrap();

    // Run diesel migrations
    let _ = conn
        .run_pending_migrations(MIGRATIONS)
        .expect("failed to run migrations");

    let (send, recv) = mpsc::channel(256);

    let _ = RUNTIME.spawn(handler(recv));

    send
}
