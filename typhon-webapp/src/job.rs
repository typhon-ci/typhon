use seed::{prelude::*, *};
use typhon_types::*;

#[derive(Clone)]
pub struct Model {
    handle: handles::Job,
    info: Option<responses::JobInfo>,
}

#[derive(Clone)]
pub enum Msg {
    Event(Event),
    FetchInfo,
    GetInfo(responses::JobInfo),
}

pub fn init(orders: &mut impl Orders<Msg>, handle: handles::Job) -> Model {
    orders.send_msg(Msg::FetchInfo);
    Model {
        handle: handle.clone(),
        info: None,
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::Event(_) => {
            orders.send_msg(Msg::FetchInfo);
        }
        Msg::FetchInfo => {
            let handle = model.handle.clone();
            orders.perform_cmd(async move {
                let req = requests::Request::Job(handle, requests::Job::Info);
                let rsp = crate::handle_request(&req).await;
                match rsp {
                    Ok(responses::Response::JobInfo(info)) => Msg::GetInfo(info),
                    _ => todo!(),
                }
            });
        }
        Msg::GetInfo(info) => {
            model.info = Some(info);
        }
    }
}

pub fn view(model: &Model, _admin: bool) -> Node<Msg> {
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
