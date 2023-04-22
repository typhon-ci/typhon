use crate::{appurl::AppUrl, perform_request, view_error};
use seed::{prelude::*, *};
use typhon_types::*;

#[derive(Clone)]
pub struct Model {
    error: Option<responses::ResponseError>,
    handle: handles::Evaluation,
    info: Option<responses::EvaluationInfo>,
    log: Option<String>,
}

impl From<Model> for AppUrl {
    fn from(m: Model) -> AppUrl {
        Vec::<String>::from(m.handle).into()
    }
}

#[derive(Clone)]
pub enum Msg {
    Cancel,
    Error(responses::ResponseError),
    ErrorIgnored,
    Event(Event),
    FetchInfo,
    FetchLog,
    GetInfo(responses::EvaluationInfo),
    GetLog(String),
    Noop,
}

pub fn init(orders: &mut impl Orders<Msg>, handle: handles::Evaluation) -> Model {
    orders.send_msg(Msg::FetchInfo);
    Model {
        error: None,
        handle: handle.clone(),
        info: None,
        log: None,
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::Cancel => {
            let handle = model.handle.clone();
            let req = requests::Request::Evaluation(handle, requests::Evaluation::Cancel);
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
            let req = requests::Request::Evaluation(handle, requests::Evaluation::Info);
            perform_request!(
                orders,
                req,
                responses::Response::EvaluationInfo(info) => Msg::GetInfo(info),
                Msg::Error,
            );
        }
        Msg::FetchLog => {
            let handle = model.handle.clone();
            let req = requests::Request::Evaluation(handle, requests::Evaluation::Log);
            perform_request!(
                orders,
                req,
                responses::Response::Log(log) => Msg::GetLog(log),
                Msg::Error,
            );
        }
        Msg::GetInfo(info) => {
            if info.status == "error" {
                orders.send_msg(Msg::FetchLog);
            }
            model.info = Some(info);
        }
        Msg::GetLog(log) => {
            model.log = Some(log);
        }
        Msg::Noop => (),
    }
}

fn view_evaluation(model: &Model) -> Node<Msg> {
    div![
        h2![
            "Evaluation",
            " ",
            a![
                &model.handle.jobset.project.project,
                attrs! {
                    At::Href => crate::Urls::project(&model.handle.jobset.project),
                },
            ],
            ":",
            a![
                &model.handle.jobset.jobset,
                attrs! {
                    At::Href => crate::Urls::jobset(&model.handle.jobset),
                },
            ],
            ":",
            model.handle.evaluation,
        ],
        match &model.info {
            None => div!["loading..."],
            Some(info) => div![
                p![format!("Status: {}", info.status)],
                p![format!("Flake locked: {}", info.flake_locked)],
                p![format!(
                    "Actions path: {}",
                    info.actions_path.clone().unwrap_or("".into())
                )],
                if info.status == "success" {
                    div![
                        h3!["Jobs"],
                        ul![info.jobs.iter().map(|job| li![a![
                            job,
                            attrs! {
                                At::Href => crate::Urls::job(
                                    &handles::Job {
                                        evaluation: model.handle.clone(),
                                        job: job.clone(),
                                    }
                                    )
                            }
                        ]])]
                    ]
                } else {
                    empty![]
                },
            ],
        },
        match &model.log {
            None => empty![],
            Some(log) => div![h3!["Log"], log],
        }
    ]
}

fn view_admin() -> Node<Msg> {
    div![
        h2!["Administration"],
        button!["Cancel", ev(Ev::Click, |_| Msg::Cancel),]
    ]
}

pub fn view(model: &Model, admin: bool) -> Node<Msg> {
    model
        .error
        .as_ref()
        .map(|err| view_error(err, Msg::ErrorIgnored))
        .unwrap_or(div![
            view_evaluation(model),
            if admin { view_admin() } else { empty![] },
        ])
}
