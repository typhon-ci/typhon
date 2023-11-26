pub mod handles {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
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

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
    pub struct Jobset {
        pub project: Project,
        pub name: String,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
    pub struct Evaluation {
        pub project: Project,
        pub num: u64,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
    pub struct Job {
        pub evaluation: Evaluation,
        pub system: String,
        pub name: String,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
    pub struct Run {
        pub job: Job,
        pub num: u64,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
    pub struct Build {
        pub drv: String,
        pub num: u64,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
    pub struct Action {
        pub project: Project,
        pub num: u64,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
    pub enum Log {
        Action(Action),
        Build(Build),
        Evaluation(Evaluation),
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
    pub enum Handle {
        Project(Project),
        Jobset(Jobset),
        Evaluation(Evaluation),
        Job(Job),
        Log(Log),
        Run(Run),
        Build(Build),
        Action(Action),
    }
    impl From<Handle> for Vec<String> {
        fn from(h: Handle) -> Self {
            match h {
                Handle::Project(h) => Self::from(h),
                Handle::Jobset(h) => Self::from(h),
                Handle::Evaluation(h) => Self::from(h),
                Handle::Job(h) => Self::from(h),
                Handle::Action(h) => Self::from(h),
                Handle::Build(h) => Self::from(h),
                Handle::Run(h) => Self::from(h),
                Handle::Log(h) => Self::from(h),
            }
        }
    }
    impl Handle {
        pub fn parent(&self) -> Option<Self> {
            Some(match self {
                Self::Project(_) => None?,
                Self::Jobset(jobset) => Handle::Project(jobset.project.clone()),
                Self::Evaluation(eval) => Handle::Project(eval.project.clone()),
                Self::Job(job) => Handle::Evaluation(job.evaluation.clone()),
                Self::Action(action) => Handle::Project(action.project.clone()),
                Self::Build(..) => None?,
                Self::Run(run) => Handle::Job(run.job.clone()),
                Self::Log(Log::Action(action)) => Handle::Action(action.clone()),
                Self::Log(Log::Build(build)) => Handle::Build(build.clone()),
                Self::Log(Log::Evaluation(eval)) => Handle::Evaluation(eval.clone()),
            })
        }
        pub fn parents(&self) -> impl Iterator<Item = Self> {
            std::iter::successors(Some(self.clone()), |current| current.parent())
        }
        pub fn path(&self) -> impl Iterator<Item = Self> {
            self.parents().collect::<Vec<_>>().into_iter().rev()
        }
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
    #[repr(u8)]
    pub enum TaskStatusKind {
        #[default]
        Pending = 0,
        Success = 1,
        Error = 2,
        Canceled = 3,
    }
    impl TryFrom<i32> for TaskStatusKind {
        type Error = ();
        fn try_from(n: i32) -> Result<TaskStatusKind, ()> {
            let arr = [Self::Pending, Self::Success, Self::Error, Self::Canceled];
            arr.get(n as usize).ok_or(()).copied()
        }
    }
    impl From<TaskStatusKind> for i32 {
        fn from(x: TaskStatusKind) -> i32 {
            (x as u8) as i32
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

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum User {
        Admin,
    }
}

pub mod requests {
    use crate::handles;

    use serde::{Deserialize, Serialize};

    pub mod search {
        use crate::data::TaskStatusKind;
        use serde::{Deserialize, Serialize};

        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub enum Request {
            Projects,
            Evaluations(Evaluation),
            Builds(Build),
            Actions(Action),
            Runs(Run),
        }

        impl std::fmt::Display for Request {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                let name = match self {
                    Self::Projects => "projects",
                    Self::Evaluations(..) => "evaluations",
                    Self::Builds(..) => "builds",
                    Self::Actions(..) => "actions",
                    Self::Runs(..) => "runs",
                };
                write!(f, "{name}")
            }
        }

        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
        pub struct Evaluation {
            pub jobset_name: Option<String>,
            pub project_name: Option<String>,
            pub status: Option<TaskStatusKind>,
        }

        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
        pub struct Build {
            pub drv: Option<String>,
            pub status: Option<TaskStatusKind>,
        }

        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
        pub struct Action {
            // TODO: allow searching actions by job
            pub name: Option<String>,
            pub project_name: Option<String>,
            pub status: Option<TaskStatusKind>,
        }

        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
        pub struct Run {
            pub evaluation_num: Option<u64>,
            pub job_name: Option<String>,
            pub job_system: Option<String>,
            pub jobset_name: Option<String>,
            pub project_name: Option<String>,
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ProjectDecl {
        pub flake: bool,
        pub url: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Project {
        //Delete,
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
        Search {
            limit: u8,
            offset: u32,
            kind: search::Request,
        },
        CreateProject {
            name: String,
            decl: ProjectDecl,
        },
        Project(handles::Project, Project),
        Jobset(handles::Jobset, Jobset),
        Evaluation(handles::Evaluation, Evaluation),
        Job(handles::Job, Job),
        Build(handles::Build, Build),
        Action(handles::Action, Action),
        Run(handles::Run, Run),
        Login {
            password: String,
        },
        User,
    }

    impl std::fmt::Display for Request {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                Request::Search { kind, .. } => write!(f, "Search through {kind}"),
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
                Request::Login { .. } => write!(f, "Log in"),
                Request::User => write!(f, "Get current user"),
            }
        }
    }
}

pub mod responses {
    use crate::data;
    use crate::data::TaskStatusKind;
    use crate::handles;

    use serde::{Deserialize, Serialize};
    use time::OffsetDateTime;

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct TimeRange {
        pub start: OffsetDateTime,
        pub end: OffsetDateTime,
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum TaskStatus {
        Pending { start: Option<OffsetDateTime> },
        Success(TimeRange),
        Error(TimeRange),
        Canceled(Option<TimeRange>),
    }
    impl From<&TaskStatus> for TaskStatusKind {
        fn from(status: &TaskStatus) -> Self {
            match status {
                TaskStatus::Pending { .. } => Self::Pending,
                TaskStatus::Success(..) => Self::Success,
                TaskStatus::Error(..) => Self::Error,
                TaskStatus::Canceled(..) => Self::Canceled,
            }
        }
    }
    impl TaskStatusKind {
        pub fn into_task_status(
            self,
            start: Option<OffsetDateTime>,
            end: Option<OffsetDateTime>,
        ) -> TaskStatus {
            let range = start.zip(end).map(|(start, end)| TimeRange { start, end });
            match self {
                Self::Pending => TaskStatus::Pending { start },
                Self::Success => TaskStatus::Success(range.expect("Broken invariant: a `TaskStatusKind::Success` needs a `time_started` and a `time_ended`")),
                Self::Error => TaskStatus::Error(range.expect("Broken invariant: a `TaskStatusKind::Error` needs a `time_started` and a `time_ended`")),
                Self::Canceled => TaskStatus::Canceled(range)
            }
        }
    }
    impl TaskStatus {
        pub fn times(self) -> (Option<OffsetDateTime>, Option<OffsetDateTime>) {
            match self {
                Self::Pending { start } => (start, None),
                Self::Success(range) | Self::Error(range) | Self::Canceled(Some(range)) => {
                    (Some(range.start), Some(range.end))
                }
                Self::Canceled(None) => (None, None),
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

    pub mod search {
        use crate::handles;
        use serde::{Deserialize, Serialize};
        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub enum Results {
            Evaluations(Vec<handles::Evaluation>),
            Builds(Vec<handles::Build>),
            Actions(Vec<handles::Action>),
            Runs(Vec<handles::Run>),
            Projects(Vec<(handles::Project, crate::responses::ProjectMetadata)>),
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Response {
        Ok,
        Search {
            total: u64,
            results: search::Results,
        },
        ProjectInfo(ProjectInfo),
        JobsetEvaluate(crate::handles::Evaluation),
        JobsetInfo(JobsetInfo),
        EvaluationInfo(EvaluationInfo),
        JobInfo(JobInfo),
        BuildInfo(BuildInfo),
        ActionInfo(ActionInfo),
        RunInfo(RunInfo),
        Log(Option<String>),
        User(Option<data::User>),
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
    //ProjectDeleted(handles::Project),
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

impl Event {
    pub fn invalidates(&self, req: &requests::Request) -> bool {
        use requests::*;
        use Event::*;
        match (self, req) {
            (
                ProjectNew(_) | ProjectUpdated(_),
                Request::Search {
                    kind: search::Request::Projects,
                    ..
                },
            ) => true,
            (ProjectUpdated(handle1), Request::Project(handle2, Project::Info)) => {
                handle1 == handle2
            }
            (ProjectUpdated(handle1), Request::Jobset(handle2, Jobset::Info)) => {
                *handle1 == handle2.project
            }
            (
                EvaluationNew(_) | EvaluationFinished(_),
                Request::Search {
                    kind: search::Request::Evaluations(_),
                    ..
                },
            ) => true,
            (EvaluationFinished(handle1), Request::Evaluation(handle2, Evaluation::Info)) => {
                handle1 == handle2
            }
            (EvaluationFinished(handle1), Request::Evaluation(handle2, Evaluation::Log)) => {
                handle1 == handle2
            }
            (
                BuildNew(_) | BuildFinished(_),
                Request::Search {
                    kind: search::Request::Builds(_),
                    ..
                },
            ) => true,
            (BuildFinished(handle1), Request::Build(handle2, Build::Info)) => handle1 == handle2,
            (BuildFinished(handle1), Request::Build(handle2, Build::Log)) => handle1 == handle2,
            (
                RunNew(_) | RunUpdated(_),
                Request::Search {
                    kind: search::Request::Runs(_),
                    ..
                },
            ) => true,
            (RunUpdated(handle1), Request::Run(handle2, Run::Info)) => handle1 == handle2,
            (
                ActionNew(_) | ActionFinished(_),
                Request::Search {
                    kind: search::Request::Actions(_),
                    ..
                },
            ) => true,
            (ActionFinished(handle1), Request::Action(handle2, Action::Info)) => handle1 == handle2,
            (ActionFinished(handle1), Request::Action(handle2, Action::Log)) => handle1 == handle2,
            (_, _) => false,
        }
    }
}
