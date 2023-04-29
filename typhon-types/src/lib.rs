pub mod handles {
    use serde::{Deserialize, Serialize};
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Project {
        pub project: String,
    }
    impl Project {
        pub fn legal(&self) -> bool {
            use lazy_static::lazy_static;
            use regex::Regex;
            lazy_static! {
                static ref RE: Regex = Regex::new("^[A-z0-9-_]+$").unwrap();
            }
            RE.is_match(&self.project)
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Jobset {
        pub project: Project,
        pub jobset: String,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Evaluation {
        pub jobset: Jobset,
        pub evaluation: i32,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Job {
        pub evaluation: Evaluation,
        pub job: String,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Build {
        pub build_hash: String,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Log {
        Evaluation(Evaluation),
        JobBegin(Job),
        JobEnd(Job),
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
            vec![x.project]
        }
    }
    impl_display!(Jobset);
    impl From<Jobset> for Vec<String> {
        fn from(x: Jobset) -> Self {
            [x.project.into(), vec![x.jobset]].concat()
        }
    }
    impl_display!(Evaluation);
    impl From<Evaluation> for Vec<String> {
        fn from(x: Evaluation) -> Self {
            [x.jobset.into(), vec![format!("{}", x.evaluation)]].concat()
        }
    }
    impl_display!(Job);
    impl From<Job> for Vec<String> {
        fn from(x: Job) -> Self {
            [x.evaluation.into(), vec![x.job]].concat()
        }
    }
    impl_display!(Build);
    impl From<Build> for Vec<String> {
        fn from(x: Build) -> Self {
            vec![x.build_hash]
        }
    }
    impl_display!(Log);
    impl From<Log> for Vec<String> {
        fn from(x: Log) -> Self {
            use Log::*;
            vec![
                match x {
                    Evaluation(_) => "evaluation",
                    JobBegin(_) => "job_begin",
                    JobEnd(_) => "job_end",
                }
                .into(),
                match x {
                    Evaluation(h) => h.to_string(),
                    JobBegin(h) => h.to_string(),
                    JobEnd(h) => h.to_string(),
                },
            ]
        }
    }

    use crate::handles as selfmod;
    pub fn project(project: String) -> Project {
        Project { project }
    }
    pub fn jobset((project, jobset): (String, String)) -> Jobset {
        Jobset {
            project: selfmod::project(project),
            jobset,
        }
    }
    pub fn evaluation((project, jobset, evaluation): (String, String, i32)) -> Evaluation {
        Evaluation {
            jobset: selfmod::jobset((project, jobset)),
            evaluation,
        }
    }
    pub fn job((project, jobset, evaluation, job): (String, String, i32, String)) -> Job {
        Job {
            evaluation: selfmod::evaluation((project, jobset, evaluation)),
            job,
        }
    }
    pub fn build(build_hash: String) -> Build {
        Build { build_hash }
    }

    #[macro_export]
    macro_rules! pattern {
        ($p:pat, $js:pat, $e:pat, $j:pat) => {
            crate::handles::Job {
                evaluation: crate::handles::pattern!($p, $js, $e),
                job: $j,
            }
        };
        ($p:pat, $js:pat, $e:pat) => {
            crate::handles::Evaluation {
                jobset: crate::handles::pattern!($p, $js),
                evaluation: $e,
            }
        };
        ($p:pat, $js:pat) => {
            crate::handles::Jobset {
                project: crate::handles::pattern!($p),
                jobset: $js,
            }
        };
        ($p:pat) => {
            crate::handles::Project { project: $p }
        };
    }

    pub use pattern;
}

pub mod requests {
    use crate::handles;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Project {
        Delete,
        Info,
        Refresh,
        SetDecl(String),
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
    pub enum Build {
        Cancel,
        Info,
        NixLog,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Request {
        ListProjects,
        CreateProject {
            handle: handles::Project,
            decl: String,
        },
        Project(handles::Project, Project),
        Jobset(handles::Jobset, Jobset),
        Evaluation(handles::Evaluation, Evaluation),
        Job(handles::Job, Job),
        Build(handles::Build, Build),
        Login(String),
    }

    impl std::fmt::Display for Request {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                Request::ListProjects => write!(f, "List projects"),
                Request::CreateProject { handle, decl } => {
                    write!(f, "Create project {handle} with declaration {decl}")
                }
                Request::Project(h, req) => write!(f, "{:?} for project {}", req, h),
                Request::Jobset(h, req) => write!(f, "{:?} for jobset {}", req, h),
                Request::Evaluation(h, req) => write!(f, "{:?} for evaluation {}", req, h),
                Request::Job(h, req) => write!(f, "{:?} for job {}", req, h),
                Request::Build(h, req) => write!(f, "{:?} for build {}", req, h),
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
        pub decl: String,
        pub decl_locked: String,
        pub jobsets: Vec<String>,
        pub metadata: ProjectMetadata,
        pub public_key: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct JobsetInfo {
        pub evaluations: Vec<(i32, i64)>,
        pub flake: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct EvaluationInfo {
        pub actions_path: Option<String>,
        pub flake_locked: String,
        pub jobs: Vec<String>,
        pub status: String,
        pub time_created: i64,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct JobInfo {
        pub build_handle: super::handles::Build,
        pub build_infos: BuildInfo,
        pub status: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct BuildInfo {
        pub drv: String,
        pub out: String,
        pub status: String,
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
        BuildInfo(BuildInfo),
        Log(String),
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
    BuildNew(handles::Build),
    BuildFinished(handles::Build),
}
