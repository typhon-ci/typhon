use crate::requests::perform_request;
use crate::widgets::evaluation_list;

use seed::{prelude::*, *};
use typhon_types::*;

struct_urls!();

pub struct Model {
    error: Option<responses::ResponseError>,
    evaluation_list: evaluation_list::Model,
    handle: handles::Jobset,
    info: Option<responses::JobsetInfo>,
    base_url: Url,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Error(responses::ResponseError),
    ErrorIgnored,
    Evaluate(bool),
    MsgEvaluationList(evaluation_list::Msg),
    Event(Event),
    FetchInfo,
    GetInfo(responses::JobsetInfo),
    Noop,
}

pub fn init(base_url: Url, orders: &mut impl Orders<Msg>, handle: handles::Jobset) -> Model {
    orders.send_msg(Msg::FetchInfo);
    Model {
        error: None,
        evaluation_list: evaluation_list::init(
            &base_url,
            &mut orders.proxy(Msg::MsgEvaluationList),
            Some(handle.project.name.clone()),
            Some(handle.name.clone()),
            1,
        ),
        handle: handle.clone(),
        info: None,
        base_url,
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
        Msg::MsgEvaluationList(msg) => {
            evaluation_list::update(
                msg,
                &mut model.evaluation_list,
                &mut orders.proxy(Msg::MsgEvaluationList),
            );
        }
        Msg::Event(event) => {
            orders.send_msg(Msg::FetchInfo);
            evaluation_list::update(
                evaluation_list::Msg::Event(event),
                &mut model.evaluation_list,
                &mut orders.proxy(Msg::MsgEvaluationList),
            );
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
    let urls = crate::Urls::new(&model.base_url);
    div![
        h2![
            "Jobset",
            " ",
            a![
                &model.handle.project.name,
                attrs! {
                    At::Href => urls.project(&model.handle.project),
                },
            ],
            ":",
            model.handle.name.clone(),
        ],
        match &model.info {
            None => div!["loading..."],
            Some(info) => div![div![
                format!("Flake: {}", info.url),
                h3!["Evaluations"],
                evaluation_list::view(&model.evaluation_list).map_msg(Msg::MsgEvaluationList),
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
    use crate::views;

    model
        .error
        .as_ref()
        .map(|err| views::error::view(&model.base_url, err, Msg::ErrorIgnored))
        .unwrap_or(div![
            view_jobset(model),
            if admin { view_admin() } else { empty![] },
        ])
}
