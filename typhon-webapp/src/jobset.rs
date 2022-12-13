use crate::{perform_request, view_error};
use seed::{prelude::*, *};
use typhon_types::*;

#[derive(Clone)]
pub struct Model {
    error: Option<responses::ResponseError>,
    handle: handles::Jobset,
    info: Option<responses::JobsetInfo>,
}

#[derive(Clone)]
pub enum Msg {
    Error(responses::ResponseError),
    ErrorIgnored,
    Evaluate(bool),
    Event(Event),
    FetchInfo,
    GetInfo(responses::JobsetInfo),
    Noop,
}

pub fn init(orders: &mut impl Orders<Msg>, handle: handles::Jobset) -> Model {
    orders.send_msg(Msg::FetchInfo);
    Model {
        error: None,
        handle: handle.clone(),
        info: None,
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::Error(err) => {
            model.error = Some(err);
        }
        Msg::ErrorIgnored => {
            model.error = None;
        }
        Msg::Evaluate(force) => {
            let handle = model.handle.clone();
            let req = requests::Request::Jobset(handle, requests::Jobset::Evaluate(force));
            perform_request!(
                orders,
                req,
                responses::Response::JobsetEvaluate(_) => Msg::Noop,
                Msg::Error,
            );
        }
        Msg::Event(_) => {
            orders.send_msg(Msg::FetchInfo);
        }
        Msg::FetchInfo => {
            let handle = model.handle.clone();
            let req = requests::Request::Jobset(handle, requests::Jobset::Info);
            perform_request!(
                orders,
                req,
                responses::Response::JobsetInfo(info) => Msg::GetInfo(info),
                Msg::Error,
            );
        }
        Msg::GetInfo(info) => {
            model.info = Some(info);
        }
        Msg::Noop => (),
    }
}

fn view_jobset(model: &Model) -> Node<Msg> {
    div![
        h2![
            "Jobset",
            " ",
            a![
                &model.handle.project.project,
                attrs! {
                    At::Href => crate::Urls::project(&model.handle.project),
                },
            ],
            ":",
            model.handle.jobset.clone(),
        ],
        match &model.info {
            None => div!["loading..."],
            Some(info) => div![div![
                format!("Flake: {}", info.flake),
                h3!["Evaluations"],
                ul![info.evaluations.iter().map(|(id, time)| li![a![
                    format!("{}", time), // TODO: format timestamp properly
                    attrs! { At::Href => crate::Urls::evaluation(
                        &handles::Evaluation{
                            jobset: model.handle.clone(),
                            evaluation: *id,
                        }
                    ) },
                ]]),]
            ]],
        },
    ]
}

fn view_admin() -> Node<Msg> {
    div![
        h3!["Administration"],
        p![button!["Evaluate", ev(Ev::Click, |_| Msg::Evaluate(false))]],
        p![button![
            "Force Evaluate",
            ev(Ev::Click, |_| Msg::Evaluate(true))
        ]],
    ]
}

pub fn view(model: &Model, admin: bool) -> Node<Msg> {
    model
        .error
        .as_ref()
        .map(|err| view_error(err, Msg::ErrorIgnored))
        .unwrap_or(div![
            view_jobset(model),
            if admin { view_admin() } else { empty![] },
        ])
}
