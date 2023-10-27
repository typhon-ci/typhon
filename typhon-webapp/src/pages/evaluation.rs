use crate::requests::perform_request;

use typhon_types::*;

use seed::{prelude::*, *};

struct_urls!();

pub struct Model {
    error: Option<responses::ResponseError>,
    handle: handles::Evaluation,
    info: Option<responses::EvaluationInfo>,
    log: Option<String>,
    base_url: Url,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Cancel,
    Error(responses::ResponseError),
    ErrorIgnored,
    Event(Event),
    FetchInfo,
    FetchLog,
    GetInfo(responses::EvaluationInfo),
    GetLog(Option<String>),
    Noop,
}

pub fn init(base_url: Url, orders: &mut impl Orders<Msg>, handle: handles::Evaluation) -> Model {
    orders.send_msg(Msg::FetchInfo);
    Model {
        error: None,
        handle: handle.clone(),
        info: None,
        log: None,
        base_url,
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
            model.log = log;
        }
        Msg::Noop => (),
    }
}

fn view_evaluation(model: &Model) -> Node<Msg> {
    let urls_1 = crate::Urls::new(&model.base_url);
    let urls_2 = crate::Urls::new(&model.base_url);
    div![
        h2![
            "Evaluation",
            " ",
            a![
                &model.handle.jobset.project.name,
                attrs! {
                    At::Href => urls_1.project(&model.handle.jobset.project),
                },
            ],
            ":",
            a![
                &model.handle.jobset.name,
                attrs! {
                    At::Href => urls_2.jobset(&model.handle.jobset),
                },
            ],
            ":",
            model.handle.num,
        ],
        match &model.info {
            None => div!["loading..."],
            Some(info) => div![
                p![format!("Status: {}", info.status)],
                p![format!("URL: {}", info.url)],
                p![format!(
                    "Actions path: {}",
                    info.actions_path.clone().unwrap_or("".into())
                )],
                if let Some(jobs) = &info.jobs {
                    div![
                        h3!["Jobs"],
                        ul![jobs.iter().map(|job| {
                            let urls = crate::Urls::new(&model.base_url);
                            li![a![
                                job.system.clone(),
                                " / ",
                                job.name.clone(),
                                attrs! {
                                    At::Href => urls.job(
                                        &handles::Job {
                                            evaluation: model.handle.clone(),
                                            system: job.system.clone(),
                                            name: job.name.clone(),
                                        })
                                }
                            ]]
                        })]
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
    use crate::views;

    model
        .error
        .as_ref()
        .map(|err| views::error::view(&model.base_url, err, Msg::ErrorIgnored))
        .unwrap_or(div![
            view_evaluation(model),
            if admin { view_admin() } else { empty![] },
        ])
}
