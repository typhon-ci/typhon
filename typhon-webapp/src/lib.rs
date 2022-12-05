mod build;
mod evaluation;
mod home;
mod job;
mod jobset;
mod login;
mod project;

use once_cell::sync::OnceCell;
use seed::{prelude::*, *};
use serde::{Deserialize, Serialize};
use typhon_types::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub client_webroot: String,
    pub server_domain: String,
    pub server_webroot: String,
    pub server_https: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            client_webroot: "".into(),
            server_domain: "127.0.0.1:8000".into(),
            server_webroot: "".into(),
            server_https: false,
        }
    }
}

pub static SETTINGS: OnceCell<Settings> = OnceCell::new();

pub fn get_password() -> Option<String> {
    LocalStorage::get("typhon_password").ok()
}

pub fn set_password(password: &String) {
    LocalStorage::insert("typhon_password", &password).expect("save password");
}

pub fn reset_password() {
    LocalStorage::remove("typhon_password").expect("remove saved password");
}

pub async fn handle_request(
    request: &requests::Request,
) -> Result<responses::Response, responses::ResponseError> {
    let settings = SETTINGS.get().unwrap();
    let password = get_password();
    let req = Request::new(format!(
        "{}://{}{}/api",
        if settings.server_https {
            "https"
        } else {
            "http"
        },
        settings.server_domain,
        settings.server_webroot
    ))
    .method(Method::Post)
    .json(request)
    .expect("Failed to serialize request");
    let req = match password {
        None => req,
        Some(pw) => req.header(Header::custom("password", pw)),
    };
    req.fetch()
        .await
        .unwrap()
        .json()
        .await
        .expect("Failed to deserialize response")
}

pub fn perform_request_aux<Ms: 'static>(
    orders: &mut impl Orders<Ms>,
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
                    log!(format!(
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
    pub fn webroot() -> Url {
        let settings = SETTINGS.get().unwrap();
        Url::new().set_path(settings.client_webroot.split('/'))
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
            .add_path_part(&handle.project)
    }
    pub fn jobset(handle: &handles::Jobset) -> Url {
        Urls::project(&handle.project)
            .add_path_part("jobsets")
            .add_path_part(&handle.jobset)
    }
    pub fn evaluation(handle: &handles::Evaluation) -> Url {
        Urls::jobset(&handle.jobset)
            .add_path_part("evaluations")
            .add_path_part(format!("{}", handle.evaluation))
    }
    pub fn job(handle: &handles::Job) -> Url {
        Urls::evaluation(&handle.evaluation)
            .add_path_part("jobs")
            .add_path_part(&handle.job)
    }
    pub fn build(handle: &handles::Build) -> Url {
        Urls::webroot()
            .add_path_part("builds")
            .add_path_part(&handle.build_hash)
    }
}

#[derive(Clone)]
enum Page {
    Login(login::Model),
    Home(home::Model),
    Project(project::Model),
    Jobset(jobset::Model),
    Evaluation(evaluation::Model),
    Job(job::Model),
    Build(build::Model),
    NotFound,
}

impl Page {
    fn init(mut url: Url, orders: &mut impl Orders<Msg>) -> Self {
        let settings = SETTINGS.get().unwrap();
        let webroot = settings.client_webroot.split('/');
        let mut path_parts = url.remaining_path_parts();
        for x in webroot {
            if x != "" {
                let y = path_parts.remove(0);
                if x != y {
                    return Page::NotFound;
                }
            }
        }
        match path_parts.as_slice() {
            [] => Page::Home(home::init(&mut orders.proxy(Msg::HomeMsg))),
            ["login"] => Page::Login(login::init(&mut orders.proxy(Msg::LoginMsg))),
            ["projects", project] => Page::Project(project::init(
                &mut orders.proxy(Msg::ProjectMsg),
                handles::project((*project).into()),
            )),
            ["projects", project, "jobsets", jobset] => Page::Jobset(jobset::init(
                &mut orders.proxy(Msg::JobsetMsg),
                handles::jobset(((*project).into(), (*jobset).into())),
            )),
            ["projects", project, "jobsets", jobset, "evaluations", evaluation] => evaluation
                .parse::<i32>()
                .map(|evaluation| {
                    Page::Evaluation(evaluation::init(
                        &mut orders.proxy(Msg::EvaluationMsg),
                        handles::evaluation(((*project).into(), (*jobset).into(), evaluation)),
                    ))
                })
                .unwrap_or(Page::NotFound),
            ["projects", project, "jobsets", jobset, "evaluations", evaluation, "jobs", job] => {
                evaluation
                    .parse::<i32>()
                    .map(|evaluation| {
                        Page::Job(job::init(
                            &mut orders.proxy(Msg::JobMsg),
                            handles::job((
                                (*project).into(),
                                (*jobset).into(),
                                evaluation,
                                (*job).into(),
                            )),
                        ))
                    })
                    .unwrap_or(Page::NotFound)
            }
            ["builds", build_hash] => Page::Build(build::init(
                &mut orders.proxy(Msg::BuildMsg),
                handles::build((*build_hash).into()),
            )),
            _ => Page::NotFound,
        }
    }
}

struct Model {
    page: Page,
    admin: bool,
    ws: WebSocket,
}

enum Msg {
    HomeMsg(home::Msg),
    LoginMsg(login::Msg),
    Logout,
    ProjectMsg(project::Msg),
    JobsetMsg(jobset::Msg),
    EvaluationMsg(evaluation::Msg),
    JobMsg(job::Msg),
    BuildMsg(build::Msg),
    UrlChanged(subs::UrlChanged),
    WsMessageReceived(WebSocketMessage),
}

fn init(url: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders.subscribe(Msg::UrlChanged);
    let msg_sender = orders.msg_sender();
    let settings = SETTINGS.get().unwrap();
    Model {
        page: Page::init(url, orders),
        admin: get_password().is_some(), // TODO
        ws: WebSocket::builder(
            format!(
                "{}://{}{}/api/events",
                if settings.server_https { "wss" } else { "ws" },
                settings.server_domain,
                settings.server_webroot
            ),
            orders,
        )
        .on_message(move |msg| {
            msg_sender(Some(Msg::WsMessageReceived(msg)));
        })
        .on_error(|| {})
        .on_close(|_| {})
        .build_and_open()
        .expect("failed to open websocket"),
    }
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match (msg, &mut *model) {
        (Msg::UrlChanged(subs::UrlChanged(url)), _) => {
            model.page = Page::init(url, orders);
        }
        (
            Msg::LoginMsg(msg),
            Model {
                page: Page::Login(login_model),
                ..
            },
        ) => match login::update(msg, login_model, &mut orders.proxy(Msg::LoginMsg)) {
            login::OutMsg::Noop => (),
            login::OutMsg::Login(pw) => {
                set_password(&pw); // TODO
                model.admin = true;
            }
        },
        (Msg::Logout, _) => {
            reset_password(); // TODO
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
            Msg::BuildMsg(msg),
            Model {
                page: Page::Build(build_model),
                ..
            },
        ) => build::update(msg, build_model, &mut orders.proxy(Msg::BuildMsg)),
        (Msg::WsMessageReceived(msg), _) => {
            let event: Event = msg.json().expect("failed to deserialize event");
            log!(event);
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
                Page::Build(model) => build::update(
                    build::Msg::Event(event),
                    model,
                    &mut orders.proxy(Msg::BuildMsg),
                ),
                _ => (),
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

fn header(model: &Model) -> Node<Msg> {
    div![
        h1!["Typhon"],
        a!["Home", attrs! { At::Href => Urls::home() }],
        " ",
        if model.admin {
            button!["Logout", ev(Ev::Click, |_| Msg::Logout)]
        } else {
            a!["Login", attrs! { At::Href => Urls::login() }]
        },
    ]
}

fn view(model: &Model) -> impl IntoNodes<Msg> {
    vec![
        header(&model),
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
            Page::Build(build_model) => {
                build::view(&build_model, model.admin).map_msg(Msg::BuildMsg)
            }
        },
    ]
}

#[wasm_bindgen]
pub fn app(settings: JsValue) {
    let settings = serde_wasm_bindgen::from_value(settings).expect("failed to parse settings");
    SETTINGS.set(settings).unwrap();
    App::start("app", init, update, view);
}
