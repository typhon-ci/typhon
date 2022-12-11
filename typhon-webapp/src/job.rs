use crate::{perform_request, view_error};
use seed::{prelude::*, *};
use typhon_types::*;

#[derive(Clone)]
pub struct Model {
    error: Option<responses::ResponseError>,
    handle: handles::Job,
    info: Option<responses::JobInfo>,
}

#[derive(Clone)]
pub enum Msg {
    Cancel,
    Error(responses::ResponseError),
    ErrorIgnored,
    Event(Event),
    FetchInfo,
    GetInfo(responses::JobInfo),
    Noop,
}

pub fn init(orders: &mut impl Orders<Msg>, handle: handles::Job) -> Model {
    orders.send_msg(Msg::FetchInfo);
    Model {
        error: None,
        handle: handle.clone(),
        info: None,
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
        Msg::GetInfo(info) => {
            model.info = Some(info);
        }
        Msg::Noop => (),
    }
}

fn view_job(model: &Model) -> Node<Msg> {
    div![
        h2![
            "Job",
            " ",
            a![
                &model.handle.evaluation.jobset.project.project,
                attrs! {
                    At::Href => crate::Urls::project(&model.handle.evaluation.jobset.project),
                },
            ],
            ":",
            a![
                &model.handle.evaluation.jobset.jobset,
                attrs! {
                    At::Href => crate::Urls::jobset(&model.handle.evaluation.jobset),
                },
            ],
            ":",
            a![
                &model.handle.evaluation.evaluation,
                attrs! {
                    At::Href => crate::Urls::evaluation(&model.handle.evaluation)
                },
            ],
            ":",
            &model.handle.job,
        ],
        match &model.info {
            None => div!["loading..."],
            Some(info) => div![
                p![
                    "Build: ",
                    a![
                        format!("{}", info.build),
                        attrs! {
                            At::Href => crate::Urls::build(&info.build)
                        },
                    ]
                ],
                p![format!("Status: {}", info.status)],
            ],
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
