mod pages;
mod requests;
mod secrets;
mod settings;
mod streams;
mod views;
mod widgets;

use pages::*;

use typhon_types::*;

use gloo_console::log;
use gloo_net::http;
use seed::{prelude::*, *};

struct_urls!();
impl<'a> Urls<'a> {
    pub fn home_urls(self) -> home::Urls<'a> {
        home::Urls::new(self.base_url())
    }
    pub fn home(self) -> Url {
        self.home_urls().base_url()
    }
    pub fn project_urls(self, project: &handles::Project) -> project::Urls<'a> {
        project::Urls::new(
            self.base_url()
                .add_path_part("projects")
                .add_path_part(project.name.clone()),
        )
    }
    pub fn project(self, project: &handles::Project) -> Url {
        self.project_urls(project).base_url()
    }
    pub fn jobset_urls(self, jobset: &handles::Jobset) -> jobset::Urls<'a> {
        jobset::Urls::new(
            self.base_url()
                .add_path_part("projects")
                .add_path_part(jobset.project.name.clone())
                .add_path_part("jobsets")
                .add_path_part(jobset.name.clone()),
        )
    }
    pub fn jobset(self, jobset: &handles::Jobset) -> Url {
        self.jobset_urls(jobset).base_url()
    }
    pub fn evaluation_urls(self, evaluation: &handles::Evaluation) -> evaluation::Urls<'a> {
        evaluation::Urls::new(
            self.base_url()
                .add_path_part("projects")
                .add_path_part(evaluation.jobset.project.name.clone())
                .add_path_part("jobsets")
                .add_path_part(evaluation.jobset.name.clone())
                .add_path_part("evaluations")
                .add_path_part(evaluation.num.to_string()),
        )
    }
    pub fn evaluation(self, evaluation: &handles::Evaluation) -> Url {
        self.evaluation_urls(evaluation).base_url()
    }
    pub fn job_urls(self, job: &handles::Job) -> job::Urls<'a> {
        job::Urls::new(
            self.base_url()
                .add_path_part("projects")
                .add_path_part(job.evaluation.jobset.project.name.clone())
                .add_path_part("jobsets")
                .add_path_part(job.evaluation.jobset.name.clone())
                .add_path_part("evaluations")
                .add_path_part(job.evaluation.num.to_string())
                .add_path_part("jobs")
                .add_path_part(job.system.clone())
                .add_path_part(job.name.clone()),
        )
    }
    pub fn job(self, job: &handles::Job) -> Url {
        self.job_urls(job).base_url()
    }
    pub fn drv_urls(self, drv: &String) -> drv::Urls<'a> {
        let mut base_url = self.base_url().add_path_part("drv");
        let path = drv.split("/");
        for part in path {
            base_url = base_url.add_path_part(part);
        }
        drv::Urls::new(base_url)
    }
    pub fn drv(self, drv: &String) -> Url {
        self.drv_urls(drv).base_url()
    }
}

enum Page {
    Login(login::Model),
    Home(home::Model),
    Project(project::Model),
    Jobset(jobset::Model),
    Evaluation(evaluation::Model),
    Job(job::Model),
    Drv(drv::Model),
    NotFound,
}

impl Page {
    fn init(mut url: Url, orders: &mut impl Orders<Msg>) -> Self {
        let base_url = url.to_base_url();
        let path_parts = url.remaining_path_parts();
        match path_parts.as_slice() {
            [] => Page::Home(home::init(base_url, &mut orders.proxy(Msg::HomeMsg))),
            ["login"] => Page::Login(login::init(
                base_url,
                &mut orders.proxy(Msg::LoginMsg),
                None,
            )),
            ["projects", project] => Page::Project(project::init(
                base_url,
                &mut orders.proxy(Msg::ProjectMsg),
                handles::project((*project).into()),
            )),
            ["projects", project, "jobsets", jobset] => Page::Jobset(jobset::init(
                base_url,
                &mut orders.proxy(Msg::JobsetMsg),
                handles::jobset(((*project).into(), (*jobset).into())),
            )),
            ["projects", project, "jobsets", jobset, "evaluations", evaluation] => evaluation
                .parse::<i64>()
                .map(|evaluation| {
                    Page::Evaluation(evaluation::init(
                        base_url,
                        &mut orders.proxy(Msg::EvaluationMsg),
                        handles::evaluation(((*project).into(), (*jobset).into(), evaluation)),
                    ))
                })
                .unwrap_or(Page::NotFound),
            ["projects", project, "jobsets", jobset, "evaluations", evaluation, "jobs", system, job] => {
                evaluation
                    .parse::<i64>()
                    .map(|evaluation| {
                        Page::Job(job::init(
                            base_url,
                            &mut orders.proxy(Msg::JobMsg),
                            handles::job((
                                (*project).into(),
                                (*jobset).into(),
                                evaluation,
                                (*system).into(),
                                (*job).into(),
                            )),
                        ))
                    })
                    .unwrap_or(Page::NotFound)
            }
            ["drv", ..] => {
                let mut drv = String::new();
                for part in path_parts.as_slice()[1..].iter() {
                    drv += &format!("/{}", part);
                }
                Page::Drv(drv::init(base_url, &mut orders.proxy(Msg::DrvMsg), &drv))
            }
            _ => Page::NotFound,
        }
    }
}

pub struct Model {
    base_url: Url,
    page: Page,
    admin: bool,
    events_handle: StreamHandle,
}

#[derive(Clone, Debug)]
pub enum Msg {
    HomeMsg(home::Msg),
    LoginMsg(login::Msg),
    Login,
    Logout,
    ProjectMsg(project::Msg),
    JobsetMsg(jobset::Msg),
    EvaluationMsg(evaluation::Msg),
    JobMsg(job::Msg),
    DrvMsg(drv::Msg),
    UrlChanged(subs::UrlChanged),
    EventsReceived(Vec<Event>),
}

fn init(url: Url, orders: &mut impl Orders<Msg>) -> Model {
    use futures::stream::StreamExt;
    orders.subscribe(Msg::UrlChanged);
    let settings = settings::Settings::load();
    let req = http::RequestBuilder::new(&format!("{}/events", settings.api_url))
        .method(http::Method::GET);
    let req = match secrets::get_token() {
        None => req,
        Some(token) => req.header(&"token", &token),
    };
    let req = req.build().unwrap();
    let events_handle =
        orders.stream_with_handle(streams::fetch_as_stream(req).map(|chunk: String| {
            let deserializer = serde_json::Deserializer::from_str(&chunk);
            let mut res: Vec<Event> = Vec::new();
            for maybe_event in deserializer.into_iter() {
                match maybe_event {
                    Ok(event) => res.push(event),
                    Err(e) => log!(format!("failed to parse event: {:?}", e)),
                }
            }
            Msg::EventsReceived(res)
        }));
    Model {
        base_url: url.to_base_url(),
        page: Page::init(url, orders),
        admin: secrets::get_token().is_some(), // TODO
        events_handle,
    }
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match (msg, &mut *model) {
        (Msg::UrlChanged(subs::UrlChanged(url)), _) => {
            model.page = Page::init(url, orders);
        }
        (Msg::Login, _) => {
            model.page = Page::Login(login::init(
                model.base_url.clone(),
                &mut orders.proxy(Msg::LoginMsg),
                Some(Url::current()),
            ))
        }
        (
            Msg::LoginMsg(msg),
            Model {
                page: Page::Login(login_model),
                ..
            },
        ) => match login::update(msg, login_model, &mut orders.proxy(Msg::LoginMsg)) {
            Some(login::OutMsg::Login(pw, url)) => {
                secrets::set_token(&pw); // TODO
                model.admin = true;
                model.page = Page::init(url.into(), orders);
            }
            None => {}
        },
        (Msg::Logout, _) => {
            secrets::reset_token(); // TODO
            model.admin = false;
        }
        (
            Msg::HomeMsg(msg),
            Model {
                page: Page::Home(home_model),
                ..
            },
        ) => home::update(msg, home_model, &mut orders.proxy(Msg::HomeMsg)),
        (
            Msg::ProjectMsg(msg),
            Model {
                page: Page::Project(project_model),
                ..
            },
        ) => project::update(msg, project_model, &mut orders.proxy(Msg::ProjectMsg)),
        (
            Msg::JobsetMsg(msg),
            Model {
                page: Page::Jobset(jobset_model),
                ..
            },
        ) => jobset::update(msg, jobset_model, &mut orders.proxy(Msg::JobsetMsg)),
        (
            Msg::EvaluationMsg(msg),
            Model {
                page: Page::Evaluation(evaluation_model),
                ..
            },
        ) => evaluation::update(msg, evaluation_model, &mut orders.proxy(Msg::EvaluationMsg)),
        (
            Msg::JobMsg(msg),
            Model {
                page: Page::Job(job_model),
                ..
            },
        ) => job::update(msg, job_model, &mut orders.proxy(Msg::JobMsg)),
        (
            Msg::DrvMsg(msg),
            Model {
                page: Page::Drv(drv_model),
                ..
            },
        ) => drv::update(msg, drv_model, &mut orders.proxy(Msg::DrvMsg)),
        (Msg::EventsReceived(mut events), _) => {
            for event in events.drain(..) {
                log!(format!("event: {:?}", event));
                match &mut model.page {
                    Page::Home(model) => home::update(
                        home::Msg::Event(event),
                        model,
                        &mut orders.proxy(Msg::HomeMsg),
                    ),
                    Page::Project(model) => project::update(
                        project::Msg::Event(event),
                        model,
                        &mut orders.proxy(Msg::ProjectMsg),
                    ),
                    Page::Jobset(model) => jobset::update(
                        jobset::Msg::Event(event),
                        model,
                        &mut orders.proxy(Msg::JobsetMsg),
                    ),
                    Page::Evaluation(model) => evaluation::update(
                        evaluation::Msg::Event(event),
                        model,
                        &mut orders.proxy(Msg::EvaluationMsg),
                    ),
                    Page::Job(model) => job::update(
                        job::Msg::Event(event),
                        model,
                        &mut orders.proxy(Msg::JobMsg),
                    ),
                    _ => (),
                }
            }
        }
        (_, _) => (),
    }
}

fn view(model: &Model) -> impl IntoNodes<Msg> {
    // the stream is canceled on the handle drop
    let _ = model.events_handle;

    nodes![
        raw!["<link href=\"/remixicon/fonts/remixicon.css\" rel=\"stylesheet\" />"],
        views::header::view(&model.base_url, model.admin, Msg::Login, Msg::Logout),
        main![
            match &model.page {
                Page::NotFound => div!["not found!"],
                Page::Home(home_model) =>
                    home::view(&home_model, model.admin).map_msg(Msg::HomeMsg),
                Page::Login(login_model) => login::view(&login_model).map_msg(Msg::LoginMsg),
                Page::Project(project_model) => {
                    project::view(&project_model, model.admin).map_msg(Msg::ProjectMsg)
                }
                Page::Jobset(jobset_model) => {
                    jobset::view(&jobset_model, model.admin).map_msg(Msg::JobsetMsg)
                }
                Page::Evaluation(evaluation_model) => {
                    evaluation::view(&evaluation_model, model.admin).map_msg(Msg::EvaluationMsg)
                }
                Page::Job(job_model) => job::view(&job_model, model.admin).map_msg(Msg::JobMsg),
                Page::Drv(drv_model) => drv::view(&drv_model).map_msg(Msg::DrvMsg),
            },
            C![match &model.page {
                Page::NotFound => "not-found",
                Page::Home(_) => "home",
                Page::Login(_) => "login",
                Page::Project(_) => "project",
                Page::Jobset(_) => "jobset",
                Page::Evaluation(_) => "evaluation",
                Page::Job(_) => "job",
                Page::Drv(_) => "drv",
            }]
        ],
    ]
}

#[wasm_bindgen]
pub fn app() {
    App::start("app", init, update, view);
}
