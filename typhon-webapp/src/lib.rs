mod appurl;
mod editable_text;
mod evaluation;
mod home;
mod job;
mod jobset;
mod login;
mod project;
mod streams;
mod timestamp;

use appurl::AppUrl;
use gloo_console::log;
use gloo_net::http;
use gloo_storage::LocalStorage;
use gloo_storage::Storage;
use once_cell::sync::OnceCell;
use seed::{prelude::*, *};
use serde::{Deserialize, Serialize};
use typhon_types::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiServerSettings {
    pub baseurl: String,
    pub https: bool,
}

impl ApiServerSettings {
    pub fn url(&self) -> String {
        format!(
            "{}://{}",
            if self.https { "https" } else { "http" },
            self.baseurl,
        )
    }
}

impl Default for ApiServerSettings {
    fn default() -> Self {
        Self {
            baseurl: "127.0.0.1:8000/api".into(),
            https: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub client_webroot: String,
    pub api_server: ApiServerSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            client_webroot: "/".into(),
            api_server: ApiServerSettings::default(),
        }
    }
}

pub static SETTINGS: OnceCell<Settings> = OnceCell::new();

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
    let settings = SETTINGS.get().unwrap();
    let token = get_token();
    let req = http::RequestBuilder::new(&settings.api_server.url()).method(http::Method::POST);
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

pub fn webroot_chunks() -> impl Iterator<Item = &'static str> {
    SETTINGS
        .get()
        .unwrap()
        .client_webroot
        .split('/')
        .filter(|chunk| !chunk.is_empty())
}

struct_urls!();
impl<'a> Urls<'a> {
    pub fn webroot() -> Url {
        Url::new().set_path(webroot_chunks())
    }
    pub fn login() -> Url {
        Urls::webroot().add_path_part("login")
    }
    pub fn home() -> Url {
        Urls::webroot()
    }
    pub fn project(handle: &handles::Project) -> Url {
        Urls::webroot()
            .add_path_part("projects")
            .add_path_part(&handle.name)
    }
    pub fn jobset(handle: &handles::Jobset) -> Url {
        Urls::project(&handle.project).add_path_part(&handle.name)
    }
    pub fn evaluation(handle: &handles::Evaluation) -> Url {
        Urls::jobset(&handle.jobset).add_path_part(format!("{}", handle.num))
    }
    pub fn job(handle: &handles::Job) -> Url {
        Urls::evaluation(&handle.evaluation)
            .add_path_part(&handle.system)
            .add_path_part(&handle.name)
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
    pub fn app_url(&self) -> AppUrl {
        match self {
            Page::Login(m) => AppUrl::from("login") + m.app_url(),
            Page::Home(_) => AppUrl::default(),
            Page::Project(m) => AppUrl::from("projects") + m.app_url(),
            Page::Jobset(m) => AppUrl::from("projects") + m.app_url(),
            Page::Evaluation(m) => AppUrl::from("projects") + m.app_url(),
            Page::Job(m) => AppUrl::from("projects") + m.app_url(),
            Page::NotFound => AppUrl::from("404"),
        }
    }

    fn from_chunks(chunks: Vec<&str>, orders: &mut impl Orders<Msg>) -> Page {
        match chunks.as_slice() {
            [] => Page::Home(home::init(&mut orders.proxy(Msg::HomeMsg))),
            ["login"] => Page::Login(login::init(&mut orders.proxy(Msg::LoginMsg), None)),
            ["projects", project] => Page::Project(project::init(
                &mut orders.proxy(Msg::ProjectMsg),
                handles::project((*project).into()),
            )),
            ["projects", project, jobset] => Page::Jobset(jobset::init(
                &mut orders.proxy(Msg::JobsetMsg),
                handles::jobset(((*project).into(), (*jobset).into())),
            )),
            ["projects", project, jobset, evaluation] => evaluation
                .parse::<i32>()
                .map(|evaluation| {
                    Page::Evaluation(evaluation::init(
                        &mut orders.proxy(Msg::EvaluationMsg),
                        handles::evaluation(((*project).into(), (*jobset).into(), evaluation)),
                    ))
                })
                .unwrap_or(Page::NotFound),
            ["projects", project, jobset, evaluation, system, job] => evaluation
                .parse::<i32>()
                .map(|evaluation| {
                    Page::Job(job::init(
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
                .unwrap_or(Page::NotFound),
            _ => Page::NotFound,
        }
    }

    fn init(mut url: Url, orders: &mut impl Orders<Msg>) -> Self {
        let webroot = webroot_chunks().collect::<Vec<_>>();
        let path_parts = url.remaining_path_parts();
        Page::from_chunks(
            path_parts
                .strip_prefix(webroot.as_slice())
                .map(|slice| slice.to_vec())
                .unwrap_or(webroot),
            orders,
        )
    }
}

pub struct Model {
    page: Page,
    admin: bool,
    events_handle: StreamHandle,
}

#[derive(Debug)]
enum Msg {
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
    let settings = SETTINGS.get().unwrap();
    let req = http::RequestBuilder::new(&format!("{}/events", settings.api_server.url()))
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
        page: Page::init(url, orders),
        admin: get_token().is_some(), // TODO
        events_handle,
    }
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    update_aux(msg, model, orders);
    let history = seed::browser::util::history();
    let url = seed::Url::from(model.page.app_url()).to_string();
    let prev = seed::browser::util::window().location().pathname().unwrap();
    if url != prev {
        log!(format!("url={url}, prev={prev}"));
        let _ = history.push_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&url));
    }
}
fn update_aux(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match (msg, &mut *model) {
        (Msg::UrlChanged(subs::UrlChanged(url)), _) => {
            model.page = Page::init(url, orders);
        }
        (Msg::Login, _) => {
            model.page = Page::Login(login::init(
                &mut orders.proxy(Msg::LoginMsg),
                Some(model.page.app_url()),
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

pub fn view_error<Ms: Clone + 'static>(err: &responses::ResponseError, msg_ignore: Ms) -> Node<Ms> {
    div![
        h2!["Error"],
        p![format!("{}", err)],
        button!["Go back", ev(Ev::Click, |_| msg_ignore)],
        a!["Home", attrs! { At::Href => Urls::home() }],
    ]
}

pub fn view_log<Ms: Clone + 'static>(log: String) -> Node<Ms> {
    tt![log.split('\n').map(|p| { p![p.replace(" ", " ")] })]
}

fn header(model: &Model) -> Node<Msg> {
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
                attrs! { At::Href => Urls::home() }
            ]
        ],
        nav![a!["Home", attrs! { At::Href => Urls::home() }],],
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
        header(model),
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
pub fn app(settings: JsValue) {
    let settings = serde_wasm_bindgen::from_value(settings).expect("failed to parse settings");
    SETTINGS.set(settings).unwrap();
    App::start("app", init, update, view);
}
