use crate::actions;
use crate::handles;
use crate::nix;
use crate::task_manager;

#[derive(Debug, derive_more::Display)]
pub enum Error {
    #[display("Access denied")]
    AccessDenied,
    #[display("Action {_0} encountered an error")]
    ActionError(actions::Error),
    #[display("Action {_0} was not found")]
    ActionNotFound(handles::Action),
    #[display("Build {_0} was not found")]
    BuildNotFound(handles::Build),
    #[display("Run {_0} was not found")]
    RunNotFound(handles::Run),
    #[display("Bad project declaration")]
    BadProjectDecl,
    #[display("Bad jobset declaration: {_0}")]
    BadJobsetDecl(String),
    #[display("Evaluation {_0} was not found")]
    EvaluationNotFound(handles::Evaluation),
    #[display("Illegal project handle: {_0}")]
    IllegalProjectHandle(handles::Project),
    #[display("Job {_0} is already running")]
    JobAlreadyRunning(handles::Job),
    #[display("Job {_0} was not found")]
    JobNotFound(handles::Job),
    #[display("Jobset {_0} was not found")]
    JobsetNotFound(handles::Jobset),
    #[display("Log {_0} was not found")]
    LogNotFound(handles::Log),
    #[display("Nix error: {_0}")]
    NixError(nix::Error),
    #[display("Project {_0} already exists")]
    ProjectAlreadyExists(handles::Project),
    #[display("Project {_0} was not found")]
    ProjectNotFound(handles::Project),
    #[display("ToDo")]
    Todo,
    #[display("Unexpected database error: {_0}")]
    UnexpectedDatabaseError(diesel::result::Error),
    #[display("Unexpected time error: {_0}")]
    UnexpectedTimeError(time::error::ComponentRange),
    #[display("Failed to log in")]
    LoginError,
    #[display("Task error: {_0}")]
    TaskError(task_manager::Error),
    #[display("{}", display_webhook_failure(_0))]
    WebhookFailure(Option<String>),
}

fn display_webhook_failure(output: &Option<String>) -> String {
    match output {
        Some(stdout) => format!("Bad webhook output: {stdout}"),
        None => "Webhook failure".to_string(),
    }
}

impl Error {
    pub fn is_internal(&self) -> bool {
        use Error::*;
        match self {
            ActionError(actions::Error::Unexpected)
            | UnexpectedDatabaseError(_)
            | UnexpectedTimeError(_)
            | TaskError(_)
            | Todo => true,
            _ => false,
        }
    }
}

impl From<diesel::result::Error> for Error {
    fn from(e: diesel::result::Error) -> Error {
        Error::UnexpectedDatabaseError(e)
    }
}

impl From<time::error::ComponentRange> for Error {
    fn from(e: time::error::ComponentRange) -> Error {
        Error::UnexpectedTimeError(e)
    }
}

impl From<nix::Error> for Error {
    fn from(e: nix::Error) -> Error {
        Error::NixError(e)
    }
}

impl From<actions::Error> for Error {
    fn from(e: actions::Error) -> Error {
        Error::ActionError(e)
    }
}

impl From<task_manager::Error> for Error {
    fn from(e: task_manager::Error) -> Error {
        Error::TaskError(e)
    }
}

impl Into<typhon_types::responses::ResponseError> for Error {
    fn into(self) -> typhon_types::responses::ResponseError {
        use {typhon_types::responses::ResponseError::*, Error::*};
        match self {
            ActionError(actions::Error::Unexpected)
            | UnexpectedDatabaseError(_)
            | UnexpectedTimeError(_)
            | TaskError(_)
            | Todo => InternalError,
            EvaluationNotFound(_)
            | JobNotFound(_)
            | JobsetNotFound(_)
            | ProjectNotFound(_)
            | ActionNotFound(_)
            | BuildNotFound(_)
            | RunNotFound(_)
            | LogNotFound(_) => ResourceNotFound(format!("{}", self)),
            AccessDenied
            | ActionError(_)
            | BadProjectDecl
            | BadJobsetDecl(_)
            | IllegalProjectHandle(_)
            | JobAlreadyRunning(_)
            | NixError(_)
            | ProjectAlreadyExists(_)
            | LoginError
            | WebhookFailure(_) => BadRequest(format!("{}", self)),
        }
    }
}
