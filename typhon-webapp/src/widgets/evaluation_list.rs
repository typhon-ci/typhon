use crate::perform_request;
use crate::view_error;
use crate::widgets::timestamp;

use seed::{prelude::*, *};
use typhon_types::*;

pub struct Model {
    base_url: Url,
    error: Option<responses::ResponseError>,
    evaluations: Vec<(handles::Evaluation, timestamp::Model)>,
    jobset_name: Option<String>,
    page: i32,
    project_name: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Msg {
    ChangePage(i32),
    Error(responses::ResponseError),
    ErrorIgnored,
    Event(Event),
    FetchEvaluations,
    GetEvaluations(Vec<(handles::Evaluation, i64)>),
    TimestampMsg(usize, timestamp::Msg),
}

pub fn init(
    base_url: &Url,
    orders: &mut impl Orders<Msg>,
    project_name: Option<String>,
    jobset_name: Option<String>,
    page: i32,
) -> Model {
    orders.send_msg(Msg::FetchEvaluations);
    Model {
        base_url: base_url.clone(),
        error: None,
        evaluations: Vec::new(),
        jobset_name,
        page,
        project_name,
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::ChangePage(page) => {
            model.page = page;
            orders.send_msg(Msg::FetchEvaluations);
        }
        Msg::Error(err) => {
            model.error = Some(err);
        }
        Msg::ErrorIgnored => {
            model.error = None;
        }
        Msg::Event(_) => {
            orders.send_msg(Msg::FetchEvaluations);
        }
        Msg::FetchEvaluations => {
            let req = requests::Request::ListEvaluations(requests::EvaluationSearch {
                jobset_name: model.jobset_name.clone(),
                limit: 10,
                offset: (model.page - 1) * 10,
                project_name: model.project_name.clone(),
            });
            perform_request!(
                orders,
                req,
                responses::Response::ListEvaluations(evaluations) => Msg::GetEvaluations(evaluations),
                Msg::Error,
            );
        }
        Msg::GetEvaluations(mut evaluations) => {
            model.evaluations = evaluations
                .drain(..)
                .enumerate()
                .map(|(i, (handle, time))| {
                    (
                        handle,
                        timestamp::init(
                            &mut orders.proxy(move |msg| Msg::TimestampMsg(i, msg)),
                            &time,
                        ),
                    )
                })
                .collect();
        }
        Msg::TimestampMsg(i, msg) => {
            if let Some((_, m)) = model.evaluations.iter_mut().nth(i as usize) {
                timestamp::update(
                    msg,
                    m,
                    &mut orders.proxy(move |msg| Msg::TimestampMsg(i, msg)),
                );
            }
        }
    }
}

pub fn view(model: &Model) -> Node<Msg> {
    if let Some(err) = &model.error {
        view_error(&model.base_url, err, Msg::ErrorIgnored)
    } else {
        div![
            ul![model
                .evaluations
                .iter()
                .enumerate()
                .map(|(i, (handle, time))| {
                    let urls = crate::Urls::new(&model.base_url);
                    li![
                        a![
                            handle.to_string(),
                            attrs! { At::Href => urls.evaluation(
                                handle
                            )},
                        ],
                        " (",
                        timestamp::view(time).map_msg(move |msg| Msg::TimestampMsg(i, msg)),
                        ")",
                    ]
                }),],
            button![
                "<",
                ev(Ev::Click, {
                    let page = model.page;
                    move |_| Msg::ChangePage(page - 1)
                })
            ],
            model.page.to_string(),
            button![
                ">",
                ev(Ev::Click, {
                    let page = model.page;
                    move |_| Msg::ChangePage(page + 1)
                })
            ],
        ]
    }
}
