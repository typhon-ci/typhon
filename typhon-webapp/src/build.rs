use crate::{perform_request, view_error};
use seed::{prelude::*, *};
use typhon_types::*;

#[derive(Clone)]
pub struct Model {
    error: Option<responses::ResponseError>,
    handle: handles::Build,
    info: Option<responses::BuildInfo>,
}

#[derive(Clone)]
pub enum Msg {
    Cancel,
    Canceled,
    Error(responses::ResponseError),
    ErrorIgnored,
    Event(Event),
    FetchInfo,
    GetInfo(responses::BuildInfo),
}

pub fn init(orders: &mut impl Orders<Msg>, handle: handles::Build) -> Model {
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
            let req = requests::Request::Build(handle, requests::Build::Cancel);
            perform_request!(
                orders,
                req,
                responses::Response::Ok => Msg::Canceled,
                Msg::Error,
            );
        }
        Msg::Canceled => {
            orders.send_msg(Msg::FetchInfo);
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
            let req = requests::Request::Build(handle, requests::Build::Info);
            perform_request!(
                orders,
                req,
                responses::Response::BuildInfo(info) => Msg::GetInfo(info),
                Msg::Error,
            );
        }
        Msg::GetInfo(info) => {
            model.info = Some(info);
        }
    }
}

fn view_build(model: &Model) -> Node<Msg> {
    div![
        h2![format!("Build {}", model.handle),],
        match &model.info {
            None => div!["loading..."],
            Some(info) => div![
                p![format!("Status: {}", info.status)],
                p![format!("Derivation: {}", info.drv)],
            ],
        },
    ]
}

fn view_admin() -> Node<Msg> {
    div![
        h2!["Administration"],
        button!["Cancel", ev(Ev::Click, |_| Msg::Cancel),]
    ]
}

pub fn view(model: &Model, admin: bool) -> Node<Msg> {
    model
        .error
        .as_ref()
        .map(|err| view_error(err, Msg::ErrorIgnored))
        .unwrap_or(div![
            view_build(model),
            if admin { view_admin() } else { empty![] },
        ])
}
