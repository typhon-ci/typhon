use crate::actions;
use crate::nix;
use crate::handles;

#[derive(Debug)]
pub enum Error {
    AccessDenied,
    ActionError(actions::Error),
    BadJobsetDecl(String),
    BuildNotFound(handles::Build),
    BuildNotRunning(handles::Build),
    EvaluationNotFound(handles::Evaluation),
    EvaluationNotRunning(handles::Evaluation),
    JobNotFound(handles::Job),
    JobsetNotFound(handles::Jobset),
    NixError(nix::Error),
    ProjectAlreadyExists(String),
    ProjectNotFound(handles::Project),
    Todo,
    UnexpectedDatabaseError(diesel::result::Error),
}

impl Error {
    pub fn is_internal(&self) -> bool {
        match self {
            Error::UnexpectedDatabaseError(_) | Error::Todo => true,
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
            BadJobsetDecl(s) => write!(f, "Bad jobset declaration: {}", s),
            BuildNotFound(build_handle) => write!(f, "Build {} not found", build_handle),
            BuildNotRunning(build_handle) => write!(f, "Build {} is not running", build_handle),
            JobNotFound(job_handle) => {
                write!(f, "Job {} not found", job_handle)
            }
            JobsetNotFound(jobset_handle) => {
                write!(f, "Jobset {} not found", jobset_handle)
            }
            EvaluationNotFound(evaluation_handle) => {
                write!(f, "Evaluation {} not found", evaluation_handle)
            }
            EvaluationNotRunning(evaluation_handle) => {
                write!(f, "Evaluation {} is not running", evaluation_handle)
            }
            ProjectAlreadyExists(project_handle) => {
                write!(f, "Project {} already exists", project_handle)
            }
            ProjectNotFound(project_handle) => write!(f, "Project {} not found", project_handle),
            NixError(e) => write!(f, "Nix error: {}", e),
            Todo => write!(f, "Unspecified error"),
            UnexpectedDatabaseError(e) => write!(f, "Database error: {}", e),
        }
    }
}

impl From<diesel::result::Error> for Error {
    fn from(e: diesel::result::Error) -> Error {
        Error::UnexpectedDatabaseError(e)
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
