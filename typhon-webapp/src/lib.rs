mod drv_log;
mod editable_text;
mod evaluation;
mod home;
mod job;
mod jobset;
mod login;
mod project;
mod streams;
mod timestamp;

use gloo_console::log;
use gloo_net::http;
use gloo_storage::LocalStorage;
use gloo_storage::Storage;
use seed::{prelude::*, *};
use serde::{Deserialize, Serialize};
use typhon_types::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub api_url: String,
}

impl Settings {
    pub fn load() -> Self {
        serde_json::from_str::<Option<Self>>(
            &document()
                .query_selector("script[id='settings']")
                .unwrap()
                .unwrap()
                .inner_html(),
        )
        .unwrap()
        .unwrap_or_default()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_url: "http://127.0.0.1:8000/api".into(),
        }
    }
}

pub fn get_token() -> Option<String> {
    LocalStorage::get("typhon_token").ok()
}

pub fn set_token(token: &String) {
    LocalStorage::set("typhon_token", &token).unwrap()
}

pub fn reset_token() {
    LocalStorage::delete("typhon_token")
}

pub async fn handle_request(
    request: &requests::Request,
) -> Result<responses::Response, responses::ResponseError> {
    let settings = Settings::load();
    let token = get_token();
    let req = http::RequestBuilder::new(&settings.api_url).method(http::Method::POST);
    let req = match token {
        None => req,
        Some(token) => req.header("token", &token),
    };
    req.json(request)
        .unwrap()
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

pub fn perform_request_aux<Ms: 'static, MsO: 'static>(
    orders: &mut impl Orders<MsO>,
    req: requests::Request,
    succ: impl FnOnce(responses::Response) -> Ms + 'static,
    err: impl FnOnce(responses::ResponseError) -> Ms + 'static,
) {
    orders.perform_cmd(async move { handle_request(&req).await.map(succ).unwrap_or_else(err) });
}

macro_rules! perform_request {
    ($orders: expr , $req: expr , $pat: pat => $body: expr , $err: expr $(,)?) => {
        let req = $req.clone();
        crate::perform_request_aux(
            $orders,
            $req,
            move |rsp| match rsp {
                $pat => $body,
                rsp => {
                    gloo_console::log!(format!(
                        "perform_request: unexpected response {:#?} to request {:#?}",
                        rsp, req
                    ));
                    $err(responses::ResponseError::InternalError)
                }
            },
            $err,
        )
    };
}

pub(crate) use perform_request;

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
}

pub enum Page {
    Login(login::Model),
    Home(home::Model),
    Project(project::Model),
    Jobset(jobset::Model),
    Evaluation(evaluation::Model),
    Job(job::Model),
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

#[derive(Debug)]
pub enum Msg {
    HomeMsg(home::Msg),
    LoginMsg(login::Msg),
    Login,
    Logout,
    ProjectMsg(project::Msg),
    JobsetMsg(jobset::Msg),
    EvaluationMsg(evaluation::Msg),
    JobMsg(job::Msg),
    UrlChanged(subs::UrlChanged),
    EventsReceived(Vec<Event>),
}

fn init(url: Url, orders: &mut impl Orders<Msg>) -> Model {
    use futures::stream::StreamExt;
    orders.subscribe(Msg::UrlChanged);
    let settings = Settings::load();
    let req = http::RequestBuilder::new(&format!("{}/events", settings.api_url))
        .method(http::Method::GET);
    let req = match get_token() {
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
        admin: get_token().is_some(), // TODO
        events_handle,
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
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
                set_token(&pw); // TODO
                model.admin = true;
                model.page = Page::init(url.into(), orders);
            }
            None => {}
        },
        (Msg::Logout, _) => {
            reset_token(); // TODO
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

pub fn view_error<Ms: Clone + 'static>(
    base_url: &Url,
    err: &responses::ResponseError,
    msg_ignore: Ms,
) -> Node<Ms> {
    let urls = Urls::new(base_url);
    div![
        h2!["Error"],
        p![format!("{}", err)],
        button!["Go back", ev(Ev::Click, |_| msg_ignore)],
        a!["Home", attrs! { At::Href => urls.home() }],
    ]
}

pub fn view_log<Ms: Clone + 'static>(log: String) -> Node<Ms> {
    tt![log.split('\n').map(|p| { p![p.replace(" ", " ")] })]
}

fn header(base_url: &Url, model: &Model) -> Node<Msg> {
    let urls_1 = Urls::new(base_url);
    let urls_2 = Urls::new(base_url);
    header![
        main![
            a![
                raw!["<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<svg width=\"12pt\" height=\"12pt\" version=\"1.1\" viewBox=\"0 0 12 12\" xmlns=\"http://www.w3.org/2000/svg\" xmlns:cc=\"http://creativecommons.org/ns#\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">
<metadata>
<rdf:RDF>
<cc:Work rdf:about=\"\">
<dc:format>image/svg+xml</dc:format>
<dc:type rdf:resource=\"http://purl.org/dc/dcmitype/StillImage\"/>
</cc:Work>
</rdf:RDF>
</metadata>
<path d=\"m10.83 4.7059c-0.47653-1.7784-2.3041-2.8336-4.0825-2.357-1.7784 0.47653-2.8336 2.3041-2.357 4.0825m1e-7 0c0.23793 0.88795 1.1533 1.4164 2.0413 1.1785 0.88795-0.23793 1.4164-1.1533 1.1785-2.0413-0.23793-0.88795-1.1533-1.4164-2.0413-1.1785-0.88795 0.23793-1.4164 1.1533-1.1785 2.0413zm-3.2198 0.86273c0.47652 1.7784 2.3041 2.8336 4.0825 2.357 1.7784-0.47653 2.8336-2.3041 2.357-4.0825\" fill=\"none\" stroke=\"#FFF\" stroke-linecap=\"round\" stroke-linejoin=\"bevel\" stroke-miterlimit=\"10\" stroke-width=\".6\"/>
</svg>"],
                span!["Typhon"],
                attrs! { At::Href => urls_1.home() }
            ]
        ],
        nav![a!["Home", attrs! { At::Href => urls_2.home() }],],
        if model.admin {
            button!["Logout", ev(Ev::Click, |_| Msg::Logout)]
        } else {
            button![a!["Login", ev(Ev::Click, |_| Msg::Login)]]
        },
    ]
}

fn view(model: &Model) -> impl IntoNodes<Msg> {
    // the stream is canceled on the handle drop
    let _ = model.events_handle;

    nodes![
        raw!["
              <link href=\"https://cdn.jsdelivr.net/npm/remixicon@3.0.0/fonts/remixicon.css\" rel=\"stylesheet\">
             "],
        header(&model.base_url, model),
        main![
            match &model.page {
                Page::NotFound => div!["not found!"],
                Page::Home(home_model) => home::view(&home_model, model.admin).map_msg(Msg::HomeMsg),
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
            }, C![
                match &model.page {
                    Page::NotFound => "not-found",
                    Page::Home(_) => "home",
                    Page::Login(_) => "login",
                    Page::Project(_) => "project",
                    Page::Jobset(_) => "jobset",
                    Page::Evaluation(_) => "evaluation",
                    Page::Job(_) => "job",
                }
            ]],
    ]
}

#[wasm_bindgen]
pub fn app() {
    App::start("app", init, update, view);
}
