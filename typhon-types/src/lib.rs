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

    use crate::handles as selfmod;
    pub const fn project(project: String) -> Project {
        Project { project }
    }
    pub const fn jobset(project: String, jobset: String) -> Jobset {
        Jobset {
            project: selfmod::project(project),
            jobset,
        }
    }
    pub const fn evaluation(project: String, jobset: String, evaluation: i32) -> Evaluation {
        Evaluation {
            jobset: selfmod::jobset(project, jobset),
            evaluation,
        }
    }
    pub const fn job(project: String, jobset: String, evaluation: i32, job: String) -> Job {
        Job {
            evaluation: selfmod::evaluation(project, jobset, evaluation),
            job,
        }
    }
    pub const fn build(build_hash: String) -> Build {
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
        Evaluate,
        Info,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Evaluation {
        Cancel,
        Info,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Job {
        Cancel,
        Info,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Build {
        Cancel,
        Info,
        Log,
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
    use std::collections::HashMap;

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct BuildInfo {
        pub drv: String,
        pub status: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct JobInfo {
        pub project: String,
        pub jobset: String,
        pub evaluation: i64,
        pub build: String,
        pub status: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ProjectMetadata {
        pub title: String,
        pub homepage: String,
        pub description: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ProjectInfo {
        pub metadata: ProjectMetadata,
        pub jobsets: Vec<String>,
        pub public_key: String,
        pub decl: String,
        pub decl_locked: String,
        pub actions_path: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct EvaluationInfo {
        pub project: String,
        pub jobset: String,
        pub locked_flake: String,
        pub time_created: i64,
        pub status: String,
        pub jobs: HashMap<String, String>,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct JobsetInfo {
        pub flake: String,
        pub evaluations: Vec<(i32, i64)>,
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
        BuildLog, // TODO
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub enum Event {
    ProjectNew(handles::Project),
    ProjectDeleted(handles::Project),
    ProjectRefreshed(handles::Project),
    ProjectJobsetsUpdated(handles::Project),
    EvaluationNew(handles::Evaluation),
    EvaluationFinished(handles::Evaluation),
    BuildNew(handles::Build),
    BuildFinished(handles::Build),
}
