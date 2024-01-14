use typhon_types::handles;

use leptos::*;
use leptos_router::{use_location, Location, ToHref};

use std::{fmt::Debug, str::FromStr};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Root<MODE: SubpageInformation = Full> {
    Login,
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
            tab: EvaluationTab::Summary,
        }
    }
}

impl From<Root> for Root<Empty> {
    fn from(e: Root) -> Self {
        match e {
            Root::Login => Root::Login,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationTab {
    Summary,
    Job(handles::Job),
    Usage,
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
        Ok(match &chunks.iter().map(|s| s.as_ref()).collect::<Vec<_>>()[..] {
            [] => Self::Projects,
            ["login"] => Self::Login,
            ["project", project] => Self::Project(handles::project(project.to_string())),
            ["project", project, "jobset", jobset] => {
                let project = project.to_string();
                let jobset = jobset.to_string();
                let handle = handles::jobset((project, jobset));
                let page = query()
                        .get("page")
                        .and_then(|p| p.parse::<u32>().ok())
                        .unwrap_or(1);
                Self::Jobset {handle, page}
            }
            ["eval", uuid, rest @ ..] if let Ok(uuid) = uuid::Uuid::from_str(uuid) => {
                let handle = handles::evaluation(uuid);
                let tab = match rest {
                    [system, name] => EvaluationTab::Job(handles::Job {
                        evaluation: handle.clone(),
                        system: system.to_string(),
                        name: name.to_string(),
                    }),
                    [] => EvaluationTab::Summary,
                    ["usage"] => EvaluationTab::Usage,
                    _ => Err(r)?,
                };
                Self::Evaluation(EvaluationPage { handle, tab })
            }
            _ => Err(r)?,
        })
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
                    EvaluationTab::Job(handle) => format!("{}/{}", handle.system, handle.name),
                    EvaluationTab::Summary => "".into(),
                    EvaluationTab::Usage => "usage".into(),
                }
            ),
        }
    }
}

use crate::components::header::*;

#[component]
pub fn Router() -> impl IntoView {
    // use crate::evaluation::Evaluation;
    // use crate::jobset::Jobset;
    // use crate::{Projects};
    let page = Signal::derive(|| Root::try_from(use_location()));
    let root_page = create_memo(move |_| page().map(Root::<Empty>::from));
    use crate::pages::*;
    let main = move || match root_page() {
        Ok(Root::Login) => view! { <Login/> },
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
                _ => EvaluationTab::Summary,
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
