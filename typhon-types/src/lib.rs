pub mod handles {
    use serde::{Deserialize, Serialize};
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Project {
        pub project: String,
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

    impl std::fmt::Display for Build {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "{}", self.build_hash)
        }
    }
    impl std::fmt::Display for Job {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "{}:{}", self.evaluation, self.job)
        }
    }
    impl std::fmt::Display for Evaluation {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "{}:{}", self.jobset, self.evaluation)
        }
    }
    impl std::fmt::Display for Jobset {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "{}:{}", self.project, self.jobset)
        }
    }
    impl std::fmt::Display for Project {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "{}", self.project)
        }
    }
    impl std::fmt::Display for Log {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            let ty = match self {
                Log::Evaluation(_) => "evaluation",
                Log::JobBegin(_) => "job_begin",
                Log::JobEnd(_) => "job_end",
            };
            let h = match self {
                Log::Evaluation(h) => h.to_string(),
                Log::JobBegin(h) => h.to_string(),
                Log::JobEnd(h) => h.to_string(),
            };
            write!(f, "{}:{}", ty, h)
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
        CreateProject(handles::Project),
        Project(handles::Project, Project),
        Jobset(handles::Jobset, Jobset),
        Evaluation(handles::Evaluation, Evaluation),
        Job(handles::Job, Job),
        Build(handles::Build, Build),
    }
}

pub mod responses {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ProjectMetadata {
        pub description: String,
        pub homepage: String,
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
        pub jobs: Vec<String>,
        pub locked_flake: String,
        pub status: String,
        pub time_created: i64,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct JobInfo {
        pub build: crate::handles::Build,
        pub status: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct BuildInfo {
        pub drv: String,
        pub status: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Response {
        Ok,
        ListProjects(Vec<String>),
        ProjectInfo(ProjectInfo),
        ProjectUpdateJobsets(Vec<String>),
        JobsetEvaluate(crate::handles::Evaluation),
        JobsetInfo(JobsetInfo),
        EvaluationInfo(EvaluationInfo),
        JobInfo(JobInfo),
        BuildInfo(BuildInfo),
        Log(String),
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
