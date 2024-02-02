use crate::actions;
use crate::handles;
use crate::nix;
use crate::task_manager;

#[derive(Debug)]
pub enum Error {
    AccessDenied,
    ActionError(actions::Error),
    ActionNotFound(handles::Action),
    BuildNotFound(handles::Build),
    RunNotFound(handles::Run),
    BadProjectDecl,
    BadJobsetDecl(String),
    EvaluationNotFound(handles::Evaluation),
    IllegalProjectHandle(handles::Project),
    JobAlreadyRunning(handles::Job),
    JobNotFound(handles::Job),
    JobsetNotFound(handles::Jobset),
    LogNotFound(handles::Log),
    NixError(nix::Error),
    ProjectAlreadyExists(handles::Project),
    ProjectNotFound(handles::Project),
    Todo,
    UnexpectedDatabaseError(diesel::result::Error),
    UnexpectedTimeError(time::error::ComponentRange),
    LoginError,
    TaskError(task_manager::Error),
    BadWebhookOutput,
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

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use Error::*;
        match self {
            AccessDenied => write!(f, "Access denied"),
            ActionError(e) => write!(f, "Action error: {}", e),
            ActionNotFound(h) => write!(f, "Action not found: {}", h),
            BuildNotFound(h) => write!(f, "Build not found: {}", h),
            RunNotFound(h) => write!(f, "Run not found: {}", h),
            BadProjectDecl => write!(f, "Bad project declaration"),
            BadJobsetDecl(s) => write!(f, "Bad jobset declaration: {}", s),
            IllegalProjectHandle(handle) => {
                write!(f, "The project name [{}] is illegal. Legal project names are sequences of alphanumerical characters that may contain dashes [-] or underscores [_].", handle.name)
            }
            JobAlreadyRunning(job_handle) => {
                write!(f, "Job {} is already running", job_handle)
            }
            JobNotFound(job_handle) => {
                write!(f, "Job {} not found", job_handle)
            }
            JobsetNotFound(jobset_handle) => {
                write!(f, "Jobset {} not found", jobset_handle)
            }
            LogNotFound(log_handle) => {
                write!(f, "Log {} not found", log_handle)
            }
            EvaluationNotFound(evaluation_handle) => {
                write!(f, "Evaluation {} not found", evaluation_handle)
            }
            ProjectAlreadyExists(project_handle) => {
                write!(f, "Project {} already exists", project_handle)
            }
            ProjectNotFound(project_handle) => write!(f, "Project {} not found", project_handle),
            NixError(e) => write!(f, "Nix error: {}", e),
            LoginError => write!(f, "Login error"),
            Todo => write!(f, "Unspecified error"),
            UnexpectedDatabaseError(e) => write!(f, "Database error: {}", e),
            UnexpectedTimeError(e) => write!(f, "Time error: {}", e),
            TaskError(e) => write!(f, "Task error: {}", e),
            BadWebhookOutput => write!(f, "Bad webhook output"),
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
            | BadWebhookOutput => BadRequest(format!("{}", self)),
        }
    }
}
