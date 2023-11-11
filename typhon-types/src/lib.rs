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
        pub project: Project,
        pub num: u64,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Job {
        pub evaluation: Evaluation,
        pub system: String,
        pub name: String,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Run {
        pub job: Job,
        pub num: u64,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Build {
        pub drv: String,
        pub num: u64,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Action {
        pub project: Project,
        pub num: u64,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Log {
        Action(Action),
        Build(Build),
        Evaluation(Evaluation),
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
            [x.project.into(), vec![x.num.to_string()]].concat()
        }
    }
    impl_display!(Job);
    impl From<Job> for Vec<String> {
        fn from(x: Job) -> Self {
            [x.evaluation.into(), vec![x.system, x.name]].concat()
        }
    }
    impl_display!(Run);
    impl From<Run> for Vec<String> {
        fn from(x: Run) -> Self {
            [x.job.into(), vec![x.num.to_string()]].concat()
        }
    }
    impl_display!(Build);
    impl From<Build> for Vec<String> {
        fn from(x: Build) -> Self {
            vec![x.drv, x.num.to_string()]
        }
    }
    impl_display!(Action);
    impl From<Action> for Vec<String> {
        fn from(x: Action) -> Self {
            [x.project.into(), vec![x.num.to_string()]].concat()
        }
    }
    impl_display!(Log);
    impl From<Log> for Vec<String> {
        fn from(x: Log) -> Self {
            use Log::*;
            vec![
                match x {
                    Action(_) => "begin",
                    Build(_) => "end",
                    Evaluation(_) => "eval",
                }
                .into(),
                match x {
                    Action(h) => h.to_string(),
                    Build(h) => h.to_string(),
                    Evaluation(h) => h.to_string(),
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
    pub fn evaluation((project, num): (String, u64)) -> Evaluation {
        Evaluation {
            project: selfmod::project(project),
            num,
        }
    }
    pub fn job((project, evaluation, system, name): (String, u64, String, String)) -> Job {
        Job {
            evaluation: selfmod::evaluation((project, evaluation)),
            system,
            name,
        }
    }
    pub fn run((project, evaluation, system, job, num): (String, u64, String, String, u64)) -> Run {
        Run {
            job: selfmod::job((project, evaluation, system, job)),
            num,
        }
    }
    pub fn build((drv, num): (String, u64)) -> Build {
        Build { drv, num }
    }
    pub fn action((project, num): (String, u64)) -> Action {
        Action {
            project: selfmod::project(project),
            num,
        }
    }
}
pub mod data {
    use serde::{Deserialize, Serialize};

    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    pub enum TaskStatusKind {
        #[default]
        Pending,
        Success,
        Error,
        Canceled,
    }
    impl TaskStatusKind {
        pub fn from_i32(i: i32) -> Self {
            match i {
                0 => Self::Pending,
                1 => Self::Success,
                2 => Self::Error,
                3 => Self::Canceled,
                _ => panic!(),
            }
        }
        pub fn to_i32(&self) -> i32 {
            match self {
                Self::Pending => 0,
                Self::Success => 1,
                Self::Error => 2,
                Self::Canceled => 3,
            }
        }
    }
    impl std::fmt::Display for TaskStatusKind {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                Self::Pending => write!(f, "pending"),
                Self::Success => write!(f, "success"),
                Self::Error => write!(f, "error"),
                Self::Canceled => write!(f, "canceled"),
            }
        }
    }
}

pub mod requests {
    use crate::data::TaskStatusKind;
    use crate::handles;

    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct EvaluationSearch {
        pub jobset_name: Option<String>,
        pub limit: u8,
        pub offset: u32,
        pub project_name: Option<String>,
        pub status: Option<TaskStatusKind>,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct BuildSearch {
        pub drv: Option<String>,
        pub limit: u8,
        pub offset: u32,
        pub status: Option<TaskStatusKind>,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ActionSearch {
        // TODO: allow searching actions by job
        pub limit: u8,
        pub name: Option<String>,
        pub offset: u32,
        pub project_name: Option<String>,
        pub status: Option<TaskStatusKind>,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct RunSearch {
        pub evaluation_num: Option<u64>,
        pub job_name: Option<String>,
        pub job_system: Option<String>,
        pub jobset_name: Option<String>,
        pub limit: u8,
        pub offset: u32,
        pub project_name: Option<String>,
    }

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
        Info,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Build {
        Info,
        Log,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Action {
        Info,
        Log,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Run {
        Cancel,
        Info,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Request {
        ListEvaluations(EvaluationSearch),
        ListBuilds(BuildSearch),
        ListActions(ActionSearch),
        ListRuns(RunSearch),
        ListProjects,
        CreateProject { name: String, decl: ProjectDecl },
        Project(handles::Project, Project),
        Jobset(handles::Jobset, Jobset),
        Evaluation(handles::Evaluation, Evaluation),
        Job(handles::Job, Job),
        Build(handles::Build, Build),
        Action(handles::Action, Action),
        Run(handles::Run, Run),
        Login(String),
    }

    impl std::fmt::Display for Request {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                Request::ListEvaluations(_) => write!(f, "Search through evaluations"),
                Request::ListBuilds(_) => write!(f, "Search through builds"),
                Request::ListActions(_) => write!(f, "Search through actions"),
                Request::ListRuns(_) => write!(f, "Search through runs"),
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
                Request::Build(h, req) => write!(f, "{:?} for build {}", req, h),
                Request::Action(h, req) => write!(f, "{:?} for action {}", req, h),
                Request::Run(h, req) => write!(f, "{:?} for run {}", req, h),
                Request::Login(_) => write!(f, "Log in"),
            }
        }
    }
}

pub mod responses {
    use crate::data::TaskStatusKind;
    use crate::handles;

    use serde::{Deserialize, Serialize};
    use time::OffsetDateTime;

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum TaskStatus {
        Pending(Option<OffsetDateTime>),
        Success(OffsetDateTime, OffsetDateTime),
        Error(OffsetDateTime, OffsetDateTime),
        Canceled(Option<(OffsetDateTime, OffsetDateTime)>),
    }
    impl TaskStatus {
        pub fn kind(&self) -> TaskStatusKind {
            match self {
                Self::Pending(_) => TaskStatusKind::Pending,
                Self::Success(_, _) => TaskStatusKind::Success,
                Self::Error(_, _) => TaskStatusKind::Error,
                Self::Canceled(_) => TaskStatusKind::Canceled,
            }
        }
        pub fn from_data(kind: i32, time_started: Option<i64>, time_finished: Option<i64>) -> Self {
            let time_started =
                time_started.map(|i| OffsetDateTime::from_unix_timestamp(i).unwrap());
            let time_finished =
                time_finished.map(|i| OffsetDateTime::from_unix_timestamp(i).unwrap());
            match TaskStatusKind::from_i32(kind) {
                TaskStatusKind::Pending => Self::Pending(time_started),
                TaskStatusKind::Success => {
                    Self::Success(time_started.unwrap(), time_finished.unwrap())
                }
                TaskStatusKind::Error => Self::Error(time_started.unwrap(), time_finished.unwrap()),
                TaskStatusKind::Canceled => Self::Canceled(
                    time_started.map(|time_started| (time_started, time_finished.unwrap())),
                ),
            }
        }
        pub fn to_data(&self) -> (i32, Option<i64>, Option<i64>) {
            let kind = self.kind().to_i32();
            match *self {
                TaskStatus::Pending(time_started) => {
                    (kind, time_started.map(|t| t.unix_timestamp()), None)
                }
                TaskStatus::Success(time_started, time_finished) => (
                    kind,
                    Some(time_started.unix_timestamp()),
                    Some(time_finished.unix_timestamp()),
                ),
                TaskStatus::Error(time_started, time_finished) => (
                    kind,
                    Some(time_started.unix_timestamp()),
                    Some(time_finished.unix_timestamp()),
                ),
                TaskStatus::Canceled(None) => (kind, None, None),
                TaskStatus::Canceled(Some((time_started, time_finished))) => (
                    kind,
                    Some(time_started.unix_timestamp()),
                    Some(time_finished.unix_timestamp()),
                ),
            }
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct SearchResult<T> {
        pub count: u8,
        pub list: Vec<T>,
        pub total: u64,
    }

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
        pub last_refresh: Option<TaskStatus>,
        pub metadata: ProjectMetadata,
        pub public_key: String,
        pub url: String,
        pub url_locked: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct JobsetInfo {
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
        pub jobs: Option<Vec<JobSystemName>>,
        pub jobset_name: String,
        pub status: TaskStatus,
        pub time_created: OffsetDateTime,
        pub url: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct JobInfo {
        pub dist: bool,
        pub drv: String,
        pub out: String,
        pub system: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct BuildInfo {
        pub drv: String,
        pub status: TaskStatus,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ActionInfo {
        pub input: String,
        pub path: String,
        pub status: TaskStatus,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct RunInfo {
        pub begin: Option<handles::Action>,
        pub build: Option<handles::Build>,
        pub end: Option<handles::Action>,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Response {
        Ok,
        ListEvaluations(SearchResult<handles::Evaluation>),
        ListBuilds(SearchResult<handles::Build>),
        ListActions(SearchResult<handles::Action>),
        ListRuns(SearchResult<handles::Run>),
        ListProjects(Vec<(String, ProjectMetadata)>),
        ProjectInfo(ProjectInfo),
        JobsetEvaluate(crate::handles::Evaluation),
        JobsetInfo(JobsetInfo),
        EvaluationInfo(EvaluationInfo),
        JobInfo(JobInfo),
        BuildInfo(BuildInfo),
        ActionInfo(ActionInfo),
        RunInfo(RunInfo),
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
    ProjectUpdated(handles::Project),
    EvaluationNew(handles::Evaluation),
    EvaluationFinished(handles::Evaluation),
    BuildNew(handles::Build),
    BuildFinished(handles::Build),
    RunNew(handles::Run),
    RunUpdated(handles::Run),
    ActionNew(handles::Action),
    ActionFinished(handles::Action),
}
