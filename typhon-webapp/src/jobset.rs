use seed::{prelude::*, *};
use typhon_types::*;

#[derive(Clone)]
pub struct Model {
    handle: handles::Jobset,
    info: Option<responses::JobsetInfo>,
}

#[derive(Clone)]
pub enum Msg {
    Evaluate,
    Evaluated,
    FetchJobsetInfo,
    GetJobsetInfo(responses::JobsetInfo),
}

pub fn init(orders: &mut impl Orders<Msg>, handle: handles::Jobset) -> Model {
    orders.send_msg(Msg::FetchJobsetInfo);
    Model {
        handle: handle.clone(),
        info: None,
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::Evaluate => {
            let handle = model.handle.clone();
            orders.perform_cmd(async move {
                let req = requests::Request::Jobset(handle, requests::Jobset::Evaluate);
                let rsp = crate::handle_request(&req).await;
                match rsp {
                    Ok(responses::Response::JobsetEvaluate(_)) => Msg::Evaluated,
                    _ => todo!(),
                }
            });
        }
        Msg::Evaluated => {
            orders.send_msg(Msg::FetchJobsetInfo);
        }
        Msg::FetchJobsetInfo => {
            let handle = model.handle.clone();
            orders.perform_cmd(async move {
                let req = requests::Request::Jobset(handle, requests::Jobset::Info);
                let rsp = crate::handle_request(&req).await;
                match rsp {
                    Ok(responses::Response::JobsetInfo(info)) => Msg::GetJobsetInfo(info),
                    _ => todo!(),
                }
            });
        }
        Msg::GetJobsetInfo(info) => {
            model.info = Some(info);
        }
    }
}

pub fn view(model: &Model, admin: bool) -> Node<Msg> {
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
        if admin {
            div![
                h3!["Administration"],
                button!["Evaluate", ev(Ev::Click, |_| Msg::Evaluate)],
            ]
        } else {
            empty![]
        }
    ]
}
