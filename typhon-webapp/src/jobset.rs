use crate::timestamp;
use crate::{appurl::AppUrl, perform_request, view_error};

use seed::{prelude::*, *};
use typhon_types::*;

pub struct Model {
    error: Option<responses::ResponseError>,
    evaluations: Vec<(i64, timestamp::Model)>,
    handle: handles::Jobset,
    info: Option<responses::JobsetInfo>,
}

impl Model {
    pub fn app_url(&self) -> AppUrl {
        Vec::<String>::from(self.handle.clone()).into()
    }
}

#[derive(Clone, Debug)]
pub enum Msg {
    Error(responses::ResponseError),
    ErrorIgnored,
    Evaluate(bool),
    Event(Event),
    FetchEvaluations,
    FetchInfo,
    GetEvaluations(Vec<(handles::Evaluation, i64)>),
    GetInfo(responses::JobsetInfo),
    Noop,
    TimestampMsg(i64, timestamp::Msg),
}

pub fn init(orders: &mut impl Orders<Msg>, handle: handles::Jobset) -> Model {
    orders.send_msg(Msg::FetchEvaluations);
    orders.send_msg(Msg::FetchInfo);
    Model {
        error: None,
        evaluations: Vec::new(),
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
            orders.send_msg(Msg::FetchEvaluations);
            orders.send_msg(Msg::FetchInfo);
        }
        Msg::FetchEvaluations => {
            let handle = model.handle.clone();
            let req = requests::Request::ListEvaluations(requests::EvaluationSearch {
                project_name: Some(handle.project.name),
                jobset_name: Some(handle.name),
                offset: 0,
                limit: 10,
            });
            perform_request!(
                orders,
                req,
                responses::Response::ListEvaluations(evaluations) => Msg::GetEvaluations(evaluations),
                Msg::Error,
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
        Msg::GetEvaluations(evaluations) => {
            model.evaluations = evaluations
                .iter()
                .map(|(handle, time)| {
                    let num = handle.num.clone();
                    (
                        num.clone(),
                        timestamp::init(
                            &mut orders.proxy(move |msg| Msg::TimestampMsg(num, msg)),
                            time,
                        ),
                    )
                })
                .collect();
        }
        Msg::GetInfo(info) => {
            model.info = Some(info);
        }
        Msg::Noop => (),
        Msg::TimestampMsg(id, msg) => {
            model
                .evaluations
                .iter_mut()
                .find(|(id1, _)| *id1 == id)
                .map(|(_, ref mut m)| {
                    let id = id.clone();
                    timestamp::update(
                        msg,
                        m,
                        &mut orders.proxy(move |msg| Msg::TimestampMsg(id, msg)),
                    )
                });
        }
    }
}

fn view_jobset(model: &Model) -> Node<Msg> {
    div![
        h2![
            "Jobset",
            " ",
            a![
                &model.handle.project.name,
                attrs! {
                    At::Href => crate::Urls::project(&model.handle.project),
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
                ul![model.evaluations.iter().map(|(num, time)| li![a![
                    timestamp::view(time).map_msg({
                        let num = num.clone();
                        move |msg| Msg::TimestampMsg(num, msg)
                    }),
                    attrs! { At::Href => crate::Urls::evaluation(
                        &handles::Evaluation{
                            jobset: model.handle.clone(),
                            num: *num,
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
