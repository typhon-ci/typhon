use seed::{prelude::*, *};
use typhon_types::*;

#[derive(Clone)]
pub struct Model {
    handle: handles::Evaluation,
    info: Option<responses::EvaluationInfo>,
}

#[derive(Clone)]
pub enum Msg {
    Cancel,
    Canceled,
    FetchEvaluationInfo,
    GetEvaluationInfo(responses::EvaluationInfo),
}

pub fn init(orders: &mut impl Orders<Msg>, handle: handles::Evaluation) -> Model {
    orders.send_msg(Msg::FetchEvaluationInfo);
    Model {
        handle: handle.clone(),
        info: None,
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::Cancel => {
            let handle = model.handle.clone();
            orders.perform_cmd(async move {
                let req = requests::Request::Evaluation(handle, requests::Evaluation::Cancel);
                let rsp = crate::handle_request(&req).await;
                match rsp {
                    Ok(responses::Response::Ok) => Msg::Canceled,
                    _ => todo!(),
                }
            });
        }
        Msg::Canceled => {
            orders.send_msg(Msg::FetchEvaluationInfo);
        }
        Msg::FetchEvaluationInfo => {
            let handle = model.handle.clone();
            orders.perform_cmd(async move {
                let req = requests::Request::Evaluation(handle, requests::Evaluation::Info);
                let rsp = crate::handle_request(&req).await;
                match rsp {
                    Ok(responses::Response::EvaluationInfo(info)) => Msg::GetEvaluationInfo(info),
                    _ => todo!(),
                }
            });
        }
        Msg::GetEvaluationInfo(info) => {
            model.info = Some(info);
        }
    }
}

pub fn view(model: &Model, admin: bool) -> Node<Msg> {
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
                p![format!("Locked flake: {}", info.locked_flake)],
                p![format!("Actions path: {}", info.actions_path)],
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
        if admin {
            div![
                h2!["Administration"],
                button!["Cancel", ev(Ev::Click, |_| Msg::Cancel),]
            ]
        } else {
            empty![]
        },
    ]
}
