use crate::get_token;
use crate::streams;
use crate::{appurl::AppUrl, perform_request, view_error, view_log, SETTINGS};

use typhon_types::*;

use gloo_net::http;
use seed::{prelude::*, *};

pub struct Model {
    error: Option<responses::ResponseError>,
    handle: handles::Job,
    info: Option<responses::JobInfo>,
    log_begin: Option<String>,
    log_end: Option<String>,
    log: Vec<String>,
}

impl Model {
    pub fn app_url(&self) -> AppUrl {
        Vec::<String>::from(self.handle.clone()).into()
    }
}

#[derive(Clone, Debug)]
pub enum Msg {
    Cancel,
    Error(responses::ResponseError),
    ErrorIgnored,
    Event(Event),
    FetchInfo,
    FetchLogBegin,
    FetchLogEnd,
    GetInfo(responses::JobInfo),
    GetLogBegin(String),
    GetLogEnd(String),
    Noop,
    LogChunk(String),
}

pub fn init(orders: &mut impl Orders<Msg>, handle: handles::Job) -> Model {
    orders.send_msg(Msg::FetchInfo);
    Model {
        error: None,
        handle: handle.clone(),
        info: None,
        log_begin: None,
        log_end: None,
        log: vec![],
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::Cancel => {
            let handle = model.handle.clone();
            let req = requests::Request::Job(handle, requests::Job::Cancel);
            perform_request!(
                orders,
                req,
                responses::Response::Ok => Msg::Noop,
                Msg::Error,
            );
        }
        Msg::Error(err) => {
            model.error = Some(err);
        }
        Msg::ErrorIgnored => {
            model.error = None;
        }
        Msg::Event(_) => {
            orders.send_msg(Msg::FetchInfo);
        }
        Msg::FetchInfo => {
            let handle = model.handle.clone();
            let req = requests::Request::Job(handle, requests::Job::Info);
            perform_request!(
                orders,
                req,
                responses::Response::JobInfo(info) => Msg::GetInfo(info),
                Msg::Error,
            );
        }
        Msg::FetchLogBegin => {
            let handle = model.handle.clone();
            let req = requests::Request::Job(handle, requests::Job::LogBegin);
            perform_request!(
                orders,
                req,
                responses::Response::Log(log) => Msg::GetLogBegin(log),
                Msg::Error,
            );
        }
        Msg::FetchLogEnd => {
            let handle = model.handle.clone();
            let req = requests::Request::Job(handle, requests::Job::LogEnd);
            perform_request!(
                orders,
                req,
                responses::Response::Log(log) => Msg::GetLogEnd(log),
                Msg::Error,
            );
        }
        Msg::GetInfo(info) => {
            if info.begin_status != "pending" {
                orders.send_msg(Msg::FetchLogBegin);
            }
            if info.end_status != "pending" {
                orders.send_msg(Msg::FetchLogEnd);
            }
            let drv = info.build_drv.clone();
            let settings = SETTINGS.get().unwrap();
            let req = http::RequestBuilder::new(&format!(
                "{}/drv-log{}",
                settings.api_server.url(),
                &drv
            ))
            .method(http::Method::GET);
            let req = match get_token() {
                None => req,
                Some(token) => req.header(&"token", &token),
            };
            let req = req.build().unwrap();
            orders
                .proxy(|chunk: String| Msg::LogChunk(chunk))
                .stream(streams::fetch_as_stream(req));
            model.info = Some(info);
        }
        Msg::GetLogBegin(log) => {
            model.log_begin = Some(log);
        }
        Msg::GetLogEnd(log) => {
            model.log_end = Some(log);
        }
        Msg::Noop => (),
        Msg::LogChunk(chunk) => model.log.push(chunk),
    }
}

fn view_job(model: &Model) -> Node<Msg> {
    div![
        h2![
            "Job",
            " ",
            a![
                &model.handle.evaluation.jobset.project.name,
                attrs! {
                    At::Href => crate::Urls::project(&model.handle.evaluation.jobset.project),
                },
            ],
            ":",
            a![
                &model.handle.evaluation.jobset.name,
                attrs! {
                    At::Href => crate::Urls::jobset(&model.handle.evaluation.jobset),
                },
            ],
            ":",
            a![
                &model.handle.evaluation.num,
                attrs! {
                    At::Href => crate::Urls::evaluation(&model.handle.evaluation)
                },
            ],
            ":",
            &model.handle.system,
            ":",
            &model.handle.name,
        ],
        match &model.info {
            None => div!["loading..."],
            Some(info) => div![
                p![format!("Drv: {}", info.build_drv)],
                p![format!("Status (begin): {}", info.begin_status)],
                p![format!("Status (build): {}", info.build_status)],
                p![format!("Status (end): {}", info.end_status)],
                if info.dist {
                    let api_url = SETTINGS.get().unwrap().api_server.url();
                    let job = &model.handle.name;
                    let system = &model.handle.system;
                    let evaluation = &model.handle.evaluation.num;
                    let jobset = &model.handle.evaluation.jobset.name;
                    let project = &model.handle.evaluation.jobset.project.name;
                    a![
                        "Dist",
                        attrs! {
                            At::Href => format!("{}/projects/{}/jobsets/{}/evaluations/{}/jobs/{}/{}/dist/index.html",
                                                api_url, project, jobset, evaluation, system, job),
                        },
                    ]
                } else {
                    empty![]
                }
            ],
        },
        code![
            &model
                .log
                .join("\n")
                .split("\n")
                .map(|line| div![line])
                .collect::<Vec<_>>(),
            style![St::Background => "#EEFFFFFF"]
        ],
        match &model.log_begin {
            None => empty![],
            Some(log) => div![h3!["Log (begin)"], view_log(log.clone()),],
        },
        match &model.log_end {
            None => empty![],
            Some(log) => div![h3!["Log (end)"], view_log(log.clone()),],
        },
    ]
}

fn view_admin() -> Node<Msg> {
    div![
        h3!["Administration"],
        p![button!["Cancel", ev(Ev::Click, |_| Msg::Cancel),]],
    ]
}

pub fn view(model: &Model, admin: bool) -> Node<Msg> {
    model
        .error
        .as_ref()
        .map(|err| view_error(err, Msg::ErrorIgnored))
        .unwrap_or(div![
            view_job(model),
            if admin { view_admin() } else { empty![] },
        ])
}
