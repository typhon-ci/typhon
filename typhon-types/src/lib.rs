pub mod handles {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Project {
        pub name: String,
    }
    impl Project {
        pub fn legal(&self) -> bool {
            use lazy_static::lazy_static;
            use regex::Regex;
            lazy_static! {
                static ref RE: Regex = Regex::new("^[A-z0-9-_]+$").unwrap();
            }
            RE.is_match(&self.name)
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Jobset {
        pub project: Project,
        pub name: String,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Evaluation {
        pub jobset: Jobset,
        pub num: i64,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Job {
        pub evaluation: Evaluation,
        pub system: String,
        pub name: String,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Log {
        Begin(Job),
        End(Job),
        Eval(Evaluation),
    }

    macro_rules! impl_display {
        ($ty:ident) => {
            impl std::fmt::Display for $ty {
                fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    write!(f, "{}", Vec::<String>::from(self.clone()).join(":"))
                }
            }
        };
    }
    impl_display!(Project);
    impl From<Project> for Vec<String> {
        fn from(x: Project) -> Self {
            vec![x.name]
        }
    }
    impl_display!(Jobset);
    impl From<Jobset> for Vec<String> {
        fn from(x: Jobset) -> Self {
            [x.project.into(), vec![x.name]].concat()
        }
    }
    impl_display!(Evaluation);
    impl From<Evaluation> for Vec<String> {
        fn from(x: Evaluation) -> Self {
            [x.jobset.into(), vec![format!("{}", x.num)]].concat()
        }
    }
    impl_display!(Job);
    impl From<Job> for Vec<String> {
        fn from(x: Job) -> Self {
            [x.evaluation.into(), vec![x.system, x.name]].concat()
        }
    }
    impl_display!(Log);
    impl From<Log> for Vec<String> {
        fn from(x: Log) -> Self {
            use Log::*;
            vec![
                match x {
                    Begin(_) => "begin",
                    End(_) => "end",
                    Eval(_) => "eval",
                }
                .into(),
                match x {
                    Begin(h) => h.to_string(),
                    End(h) => h.to_string(),
                    Eval(h) => h.to_string(),
                },
            ]
        }
    }

    use crate::handles as selfmod;
    pub fn project(name: String) -> Project {
        Project { name }
    }
    pub fn jobset((project, name): (String, String)) -> Jobset {
        Jobset {
            project: selfmod::project(project),
            name,
        }
    }
    pub fn evaluation((project, jobset, num): (String, String, i64)) -> Evaluation {
        Evaluation {
            jobset: selfmod::jobset((project, jobset)),
            num,
        }
    }
    pub fn job(
        (project, jobset, evaluation, system, name): (String, String, i64, String, String),
    ) -> Job {
        Job {
            evaluation: selfmod::evaluation((project, jobset, evaluation)),
            system,
            name,
        }
    }
}

pub mod requests {
    use crate::handles;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ProjectDecl {
        pub flake: bool,
        pub url: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Project {
        Delete,
        Info,
        Refresh,
        SetDecl(ProjectDecl),
        SetPrivateKey(String),
        UpdateJobsets,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Jobset {
        Evaluate(bool),
        Info,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Evaluation {
        Cancel,
        Info,
        Log,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Job {
        Cancel,
        Info,
        LogBegin,
        LogEnd,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Request {
        ListProjects,
        CreateProject { name: String, decl: ProjectDecl },
        Project(handles::Project, Project),
        Jobset(handles::Jobset, Jobset),
        Evaluation(handles::Evaluation, Evaluation),
        Job(handles::Job, Job),
        Login(String),
    }

    impl std::fmt::Display for Request {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                Request::ListProjects => write!(f, "List projects"),
                Request::CreateProject { name, decl } => {
                    write!(
                        f,
                        "Create{} project {} with url {}",
                        if !decl.flake { " legacy" } else { "" },
                        name,
                        decl.url
                    )
                }
                Request::Project(h, req) => write!(f, "{:?} for project {}", req, h),
                Request::Jobset(h, req) => write!(f, "{:?} for jobset {}", req, h),
                Request::Evaluation(h, req) => write!(f, "{:?} for evaluation {}", req, h),
                Request::Job(h, req) => write!(f, "{:?} for job {}", req, h),
                Request::Login(_) => write!(f, "Log in"),
            }
        }
    }
}

pub mod responses {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ProjectMetadata {
        #[serde(default)]
        pub description: String,
        #[serde(default)]
        pub homepage: String,
        #[serde(default)]
        pub title: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ProjectInfo {
        pub actions_path: Option<String>,
        pub flake: bool,
        pub jobsets: Vec<String>,
        pub metadata: ProjectMetadata,
        pub public_key: String,
        pub url: String,
        pub url_locked: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct JobsetInfo {
        pub evaluations: Vec<(i64, i64)>,
        pub flake: bool,
        pub url: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct JobSystemName {
        pub system: String,
        pub name: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct EvaluationInfo {
        pub actions_path: Option<String>,
        pub flake: bool,
        pub jobs: Vec<JobSystemName>,
        pub status: String,
        pub time_created: i64,
        pub time_finished: Option<i64>,
        pub url: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct JobInfo {
        pub begin_status: String,
        pub begin_time_finished: Option<i64>,
        pub begin_time_started: Option<i64>,
        pub build_drv: String,
        pub build_out: String,
        pub build_status: String,
        pub build_time_finished: Option<i64>,
        pub build_time_started: Option<i64>,
        pub dist: bool,
        pub end_status: String,
        pub end_time_finished: Option<i64>,
        pub end_time_started: Option<i64>,
        pub system: String,
        pub time_created: i64,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Response {
        Ok,
        ListProjects(Vec<(String, ProjectMetadata)>),
        ProjectInfo(ProjectInfo),
        ProjectUpdateJobsets(Vec<String>),
        JobsetEvaluate(crate::handles::Evaluation),
        JobsetInfo(JobsetInfo),
        EvaluationInfo(EvaluationInfo),
        JobInfo(JobInfo),
        Log(Option<String>),
        Login { token: String },
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum ResponseError {
        BadRequest(String),
        InternalError,
        ResourceNotFound(String),
    }

    impl std::fmt::Display for ResponseError {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                ResponseError::BadRequest(e) => write!(f, "Bad request: {}", e),
                ResponseError::InternalError => write!(f, "Internal server error"),
                ResponseError::ResourceNotFound(e) => write!(f, "Resource not found: {}", e),
            }
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Event {
    ProjectNew(handles::Project),
    ProjectDeleted(handles::Project),
    ProjectJobsetsUpdated(handles::Project),
    ProjectUpdated(handles::Project),
    EvaluationNew(handles::Evaluation),
    EvaluationFinished(handles::Evaluation),
    JobUpdated(handles::Job),
}
