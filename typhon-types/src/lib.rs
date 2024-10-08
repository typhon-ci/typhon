mod task_status;

pub mod handles {
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
    #[serde(transparent)]
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
    #[serde(transparent)]
    pub struct Evaluation {
        pub uuid: Uuid,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
    pub struct Job {
        pub evaluation: Evaluation,
        pub name: String,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
    pub struct Run {
        #[serde(flatten)]
        pub job: Job,
        pub num: u32,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
    #[serde(transparent)]
    pub struct Build {
        pub uuid: Uuid,
    }
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
    #[serde(transparent)]
    pub struct Action {
        pub uuid: Uuid,
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
                Self::Evaluation(_) => None?,
                Self::Job(job) => Handle::Evaluation(job.evaluation.clone()),
                Self::Action(_) => None?,
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
            vec![x.uuid.to_string()]
        }
    }
    impl_display!(Job);
    impl From<Job> for Vec<String> {
        fn from(x: Job) -> Self {
            [x.evaluation.into(), vec![x.name]].concat()
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
            vec![x.uuid.to_string()]
        }
    }
    impl_display!(Action);
    impl From<Action> for Vec<String> {
        fn from(x: Action) -> Self {
            vec![x.uuid.to_string()]
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
    pub fn evaluation(uuid: Uuid) -> Evaluation {
        Evaluation { uuid }
    }
    pub fn job((evaluation, name): (Uuid, String)) -> Job {
        Job {
            evaluation: selfmod::evaluation(evaluation),
            name,
        }
    }
    pub fn run((evaluation, job, num): (Uuid, String, u32)) -> Run {
        Run {
            job: selfmod::job((evaluation, job)),
            num,
        }
    }
    pub fn build(uuid: Uuid) -> Build {
        Build { uuid }
    }
    pub fn action(uuid: Uuid) -> Action {
        Action { uuid }
    }
}
pub mod data {
    pub use crate::task_status::TaskStatusKind;
    use serde::{Deserialize, Serialize};

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
        use uuid::Uuid;

        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, derive_more::Display)]
        #[serde(rename_all = "lowercase")]
        #[serde(tag = "type")]
        pub enum Kind {
            #[display("projects")]
            Projects,
            #[display("jobsets")]
            Jobsets(Jobset),
            #[display("evaluations")]
            Evaluations(Evaluation),
            #[display("builds")]
            Builds(Build),
            #[display("actions")]
            Actions(Action),
            #[display("runs")]
            Runs(Run),
        }

        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub struct Request {
            pub limit: u8,
            pub offset: u32,
            #[serde(flatten)]
            pub kind: Kind,
        }

        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
        pub struct Jobset {
            pub project_name: Option<String>,
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
            pub evaluation_uuid: Option<Uuid>,
            pub job_name: Option<String>,
            pub jobset_name: Option<String>,
            pub project_name: Option<String>,
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct JobsetDecl {
        pub flake: bool,
        pub url: String,
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
        UpdateJobsets,
        NewJobset { name: String, decl: JobsetDecl },
        DeleteJobset { name: String },
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
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Job {
        Info,
        Rerun,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Build {
        Info,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Action {
        Info,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Run {
        //Cancel,
        Info,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, derive_more::Display)]
    pub enum Request {
        #[display("Search through {}", _0.kind)]
        Search(search::Request),
        #[display("Create project {name} with url {} (flake: {})", decl.url, decl.flake)]
        CreateProject { name: String, decl: ProjectDecl },
        #[display("{_1:?} for project {_0}")]
        Project(handles::Project, Project),
        #[display("{_1:?} for jobset {_0}")]
        Jobset(handles::Jobset, Jobset),
        #[display("{_1:?} for evaluation {_0}")]
        Evaluation(handles::Evaluation, Evaluation),
        #[display("{_1:?} for job {_0}")]
        Job(handles::Job, Job),
        #[display("{_1:?} for build {_0}")]
        Build(handles::Build, Build),
        #[display("{_1:?} for action {_0}")]
        Action(handles::Action, Action),
        #[display("{_1:?} for run {_0}")]
        Run(handles::Run, Run),
        #[display("Log in")]
        Login { password: String },
        #[display("Get current user")]
        User,
    }
}

pub mod responses {
    use crate::data;
    use crate::handles;
    use std::collections::HashMap;

    pub use crate::task_status::{TaskStatus, TaskStatusKind, TimeRange};
    use serde::{Deserialize, Serialize};
    use time::OffsetDateTime;

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
        pub handle: handles::Project,
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
        pub handle: handles::Jobset,
        pub flake: bool,
        pub url: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct EvaluationInfo {
        pub handle: handles::Evaluation,
        pub actions_path: Option<String>,
        pub flake: bool,
        pub jobs: HashMap<String, JobInfo>,
        pub jobset_name: String,
        pub project: handles::Project,
        pub status: TaskStatus,
        #[serde(with = "time::serde::timestamp")]
        pub time_created: OffsetDateTime,
        pub url: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct JobInfo {
        pub handle: handles::Job,
        pub dist: bool,
        pub drv: String,
        pub out: String,
        pub last_run: RunInfo,
        pub run_count: u32,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct BuildInfo {
        pub handle: handles::Build,
        pub drv: String,
        pub status: TaskStatus,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ActionInfo {
        pub handle: handles::Action,
        pub input: String,
        pub name: String,
        pub path: String,
        pub project: handles::Project,
        pub status: TaskStatus,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct RunInfo {
        pub handle: handles::Run,
        pub begin: Option<ActionInfo>,
        pub build: Option<BuildInfo>,
        pub end: Option<ActionInfo>,
    }

    pub mod search {
        use crate::handles;
        use serde::{Deserialize, Serialize};
        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub enum Results {
            Evaluations(Vec<handles::Evaluation>),
            Jobsets(Vec<handles::Jobset>),
            Builds(Vec<handles::Build>),
            Actions(Vec<handles::Action>),
            Runs(Vec<handles::Run>),
            Projects(Vec<(handles::Project, crate::responses::ProjectMetadata)>),
        }
        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub struct Info {
            pub total: u32,
            pub results: Results,
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Response {
        Ok,
        Search(search::Info),
        ProjectInfo(ProjectInfo),
        JobsetEvaluate(crate::handles::Evaluation),
        JobsetInfo(JobsetInfo),
        EvaluationInfo(EvaluationInfo),
        JobInfo(JobInfo),
        BuildInfo(BuildInfo),
        ActionInfo(ActionInfo),
        RunInfo(RunInfo),
        User(Option<data::User>),
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, derive_more::Display)]
    pub enum ResponseError {
        #[display("Bad request: {_0}")]
        BadRequest(String),
        #[display("Internal server error")]
        InternalError,
        #[display("Resource not found: {_0}")]
        ResourceNotFound(String),
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Event {
    Ping,
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
        use requests::{Request as Req, *};
        use Event as Ev;
        match (self, req) {
            (_, Req::Search(requests::search::Request { kind, .. })) => {
                use search::Kind as Search;
                match (kind, self) {
                    (Search::Projects, Ev::ProjectNew(_) | Ev::ProjectUpdated(_))
                    | (Search::Evaluations(_), Ev::EvaluationNew(_) | Ev::EvaluationFinished(_))
                    | (Search::Runs(_), Ev::RunUpdated(_) | Ev::RunNew(_))
                    | (Search::Builds(_), Ev::BuildNew(_) | Ev::BuildFinished(_))
                    | (Search::Actions(_), Ev::ActionNew(_) | Ev::ActionFinished(_)) => true,
                    _ => false,
                }
            }
            (Ev::ProjectUpdated(h1), Req::Project(h2, Project::Info)) => h1 == h2,
            (Ev::ProjectUpdated(h1), Req::Jobset(h2, Jobset::Info)) => *h1 == h2.project,
            (Ev::EvaluationFinished(h1), Req::Evaluation(h2, Evaluation::Info)) => h1 == h2,
            (Ev::BuildFinished(h1), Req::Build(h2, Build::Info)) => h1 == h2,
            (Ev::RunUpdated(h1), Req::Run(h2, Run::Info)) => h1 == h2,
            (Ev::ActionFinished(h1), Req::Action(h2, Action::Info)) => h1 == h2,
            (_, _) => false,
        }
    }
}
