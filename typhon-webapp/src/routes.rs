use typhon_types::handles;

use leptos::*;
use leptos_router::{use_location, Location, ToHref};

use std::{fmt::Debug, str::FromStr};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Root<MODE: SubpageInformation = Full> {
    Login,
    Dashboard {
        tab: DashboardTab,
        page: u32,
    },
    Projects,
    Project(handles::Project),
    Jobset {
        handle: handles::Jobset,
        page: MODE::PageNum,
    },
    Evaluation(EvaluationPage<MODE>),
}
pub trait SubpageInformation: Copy + Clone + Debug + Eq {
    type EvaluationTab: Clone + Debug + Eq;
    type PageNum: Clone + Debug + Eq;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Empty;
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Full;

impl SubpageInformation for Empty {
    type EvaluationTab = ();
    type PageNum = ();
}

impl SubpageInformation for Full {
    type EvaluationTab = EvaluationTab;
    type PageNum = u32;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluationPage<MODE: SubpageInformation = Full> {
    pub handle: handles::Evaluation,
    pub tab: MODE::EvaluationTab,
}

impl From<EvaluationPage> for Root {
    fn from(e: EvaluationPage) -> Self {
        Root::Evaluation(e)
    }
}

impl From<EvaluationPage> for EvaluationPage<Empty> {
    fn from(e: EvaluationPage) -> Self {
        Self {
            handle: e.handle,
            tab: (),
        }
    }
}

impl From<EvaluationPage<Empty>> for EvaluationPage {
    fn from(e: EvaluationPage<Empty>) -> Self {
        Self {
            handle: e.handle,
            tab: EvaluationTab::Info,
        }
    }
}

impl From<Root> for Root<Empty> {
    fn from(e: Root) -> Self {
        match e {
            Root::Login => Root::Login,
            Root::Dashboard { tab, page } => Root::Dashboard { tab, page },
            Root::Projects => Root::Projects,
            Root::Project(h) => Root::Project(h),
            Root::Jobset { handle, .. } => Root::Jobset { handle, page: () },
            Root::Evaluation(e) => Root::Evaluation(e.into()),
        }
    }
}

impl From<Root<Empty>> for Root {
    fn from(e: Root<Empty>) -> Self {
        match e {
            Root::Login => Root::Login,
            Root::Dashboard { tab, page } => Root::Dashboard { tab, page },
            Root::Projects => Root::Projects,
            Root::Project(h) => Root::Project(h),
            Root::Jobset { handle, .. } => Root::Jobset { handle, page: 1 },
            Root::Evaluation(e) => Root::Evaluation(e.into()),
        }
    }
}

impl From<Root<Empty>> for Option<handles::Handle> {
    fn from(e: Root<Empty>) -> Self {
        Some(match e {
            Root::Login => None?,
            Root::Dashboard { .. } => None?,
            Root::Projects => None?,
            Root::Project(handle) => handles::Handle::Project(handle),
            Root::Jobset { handle, .. } => handles::Handle::Jobset(handle),
            Root::Evaluation(eval) => handles::Handle::Evaluation(eval.handle),
        })
    }
}

impl From<handles::Handle> for Root<Empty> {
    fn from(e: handles::Handle) -> Self {
        match e {
            handles::Handle::Project(handle) => Root::Project(handle),
            handles::Handle::Jobset(handle) => Root::Jobset { handle, page: () },
            handles::Handle::Evaluation(handle) => {
                Root::Evaluation(EvaluationPage { handle, tab: () })
            }
            _ => panic!(),
        }
    }
}

impl From<handles::Handle> for Root {
    fn from(e: handles::Handle) -> Self {
        Root::<Empty>::from(e).into()
    }
}

pub fn to_url<T>(x: T) -> String
where
    Root: From<T>,
{
    String::from(Root::from(x))
}

impl ToHref for Root {
    fn to_href(&self) -> Box<dyn Fn() -> String + '_> {
        Box::new(|| String::from(self.clone()))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum LogTab {
    Begin,
    End,
    Build,
}

impl Default for LogTab {
    fn default() -> Self {
        LogTab::Build
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DashboardTab {
    Evaluations,
    Builds,
    Actions,
}

impl std::fmt::Display for DashboardTab {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let x = match self {
            DashboardTab::Evaluations => "evaluations",
            DashboardTab::Builds => "builds",
            DashboardTab::Actions => "actions",
        };
        write!(f, "{x}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationTab {
    Info,
    Job {
        handle: handles::Job,
        log_tab: LogTab,
    },
}

impl EvaluationTab {
    pub fn drop_log_tab(&self) -> Self {
        match self {
            Self::Job { handle, .. } => Self::Job {
                handle: handle.clone(),
                log_tab: LogTab::default(),
            },
            _ => self.clone(),
        }
    }
}

impl TryFrom<Location> for Root {
    type Error = Location;
    fn try_from(r: Location) -> Result<Self, Self::Error> {
        let Location {
            pathname,
            // search,
            query,
            // hash,
            ..
        } = &r;
        let pathname = pathname.get();
        let chunks: Vec<_> = pathname
            .split("/")
            .filter(|s| !s.is_empty())
            .map(|s| urlencoding::decode(s).map(|s| s.to_string()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| r.clone())?;
        Ok(
            match &chunks.iter().map(|s| s.as_ref()).collect::<Vec<_>>()[..] {
                [] => Self::Projects,
                ["login"] => Self::Login,
                ["dashboard"] => Self::Dashboard {
                    tab: DashboardTab::Builds,
                    page: 1,
                },
                ["dashboard", tab] => {
                    let tab = match tab {
                        &"evaluations" => DashboardTab::Evaluations,
                        &"builds" => DashboardTab::Builds,
                        &"actions" => DashboardTab::Actions,
                        _ => Err(r.clone())?,
                    };
                    let page = query()
                        .get("page")
                        .and_then(|p| p.parse::<u32>().ok())
                        .unwrap_or(1);
                    if page < 1 {
                        Err(r)?
                    }
                    Self::Dashboard { tab, page }
                }
                ["project", project] => Self::Project(handles::project(project.to_string())),
                ["project", project, "jobset", jobset] => {
                    let project = project.to_string();
                    let jobset = jobset.to_string();
                    let handle = handles::jobset((project, jobset));
                    let page = query()
                        .get("page")
                        .and_then(|p| p.parse::<u32>().ok())
                        .unwrap_or(1);
                    if page < 1 {
                        Err(r)?
                    }
                    Self::Jobset { handle, page }
                }
                ["evaluation", uuid, rest @ ..] if let Ok(uuid) = uuid::Uuid::from_str(uuid) => {
                    let handle = handles::evaluation(uuid);
                    let tab = match rest {
                        [system, name, log_tab @ ..] => {
                            let handle = handles::Job {
                                evaluation: handle.clone(),
                                system: system.to_string(),
                                name: name.to_string(),
                            };
                            let log_tab = match log_tab {
                                [] => LogTab::default(),
                                [log_tab] => match LogTab::from_str(log_tab) {
                                    Ok(log_tab) => log_tab,
                                    Err(_) => Err(r)?,
                                },
                                _ => Err(r)?,
                            };
                            EvaluationTab::Job { handle, log_tab }
                        }
                        [] => EvaluationTab::Info,
                        _ => Err(r)?,
                    };
                    Self::Evaluation(EvaluationPage { handle, tab })
                }
                _ => Err(r)?,
            },
        )
    }
}

impl From<Root> for String {
    fn from(r: Root) -> Self {
        fn path<T: Into<Vec<String>>>(handle: T) -> String {
            let vec: Vec<String> = handle.into();
            vec.iter()
                .map(|s| urlencoding::encode(s).to_string())
                .collect::<Vec<_>>()
                .join("/")
        }
        match r {
            Root::Login => "/login".to_string(),
            Root::Dashboard { tab, page } => format!("/dashboard/{}?page={page}", tab),
            Root::Projects => "".to_string(),
            Root::Project(handle) => format!("/project/{}", path(handle)),
            Root::Jobset { handle, page } => format!(
                "/project/{}/jobset/{}?page={page}",
                handle.project.name, handle.name
            ),
            Root::Evaluation(e) => format!(
                "/eval/{}/{}",
                e.handle.uuid,
                match e.tab {
                    EvaluationTab::Job { handle, log_tab } => {
                        let log_tab: &str = log_tab.into();
                        format!("{}/{}/{}", handle.system, handle.name, log_tab)
                    }
                    EvaluationTab::Info => "".into(),
                }
            ),
        }
    }
}

use crate::components::header::*;

#[component]
pub fn Router() -> impl IntoView {
    let page = Signal::derive(|| Root::try_from(use_location()));
    let root_page = create_memo(move |_| page().map(Root::<Empty>::from));
    use crate::pages::*;
    let main = move || match root_page() {
        Ok(Root::Login) => view! { <Login/> },
        Ok(Root::Dashboard { tab, page }) => {
            view! { <Dashboard tab page/> }
        }
        Ok(Root::Projects) => view! { <Projects/> },
        Ok(Root::Project(handle)) => {
            view! { <Project handle/> }
        }
        Ok(Root::Jobset { handle, .. }) => {
            let page = create_memo(move |_| match page() {
                Ok(Root::Jobset { page, .. }) => page,
                _ => 1,
            });
            view! { <Jobset handle page/> }
        }
        Ok(Root::Evaluation(e)) => {
            let handle = Signal::derive(move || e.handle.clone());
            let tab = create_memo(move |_| match page() {
                Ok(Root::Evaluation(e)) => e.tab,
                _ => EvaluationTab::Info,
            });
            view! { <Evaluation handle tab/> }
        }
        Err(loc) => format!("Unknow view: {:#?}", loc).into_view(),
    };
    let route = Signal::derive(move || root_page().ok());
    view! {
        <Header route/>
        <main>{main}</main>
    }
}
