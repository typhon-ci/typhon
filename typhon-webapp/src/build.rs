use seed::{prelude::*, *};
use typhon_types::*;

#[derive(Clone)]
pub struct Model {
    handle: handles::Build,
    info: Option<responses::BuildInfo>,
}

#[derive(Clone)]
pub enum Msg {
    Cancel,
    Canceled,
    Event(Event),
    FetchInfo,
    GetInfo(responses::BuildInfo),
}

pub fn init(orders: &mut impl Orders<Msg>, handle: handles::Build) -> Model {
    orders.send_msg(Msg::FetchInfo);
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
                let req = requests::Request::Build(handle, requests::Build::Cancel);
                let rsp = crate::handle_request(&req).await;
                match rsp {
                    Ok(responses::Response::Ok) => Msg::Canceled,
                    _ => todo!(),
                }
            });
        }
        Msg::Canceled => {
            orders.send_msg(Msg::FetchInfo);
        }
        Msg::Event(_) => {
            orders.send_msg(Msg::FetchInfo);
        }
        Msg::FetchInfo => {
            let handle = model.handle.clone();
            orders.perform_cmd(async move {
                let req = requests::Request::Build(handle, requests::Build::Info);
                let rsp = crate::handle_request(&req).await;
                match rsp {
                    Ok(responses::Response::BuildInfo(info)) => Msg::GetInfo(info),
                    _ => todo!(),
                }
            });
        }
        Msg::GetInfo(info) => {
            model.info = Some(info);
        }
    }
}

pub fn view(model: &Model, admin: bool) -> Node<Msg> {
    div![
        h2![format!("Build {}", model.handle),],
        match &model.info {
            None => div!["loading..."],
            Some(info) => div![
                p![format!("Status: {}", info.status)],
                p![format!("Derivation: {}", info.drv)],
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
