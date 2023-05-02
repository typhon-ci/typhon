use crate::{appurl::AppUrl, perform_request, view_error, view_log, SETTINGS};
use seed::{prelude::*, *};
use typhon_types::*;

#[derive(Clone)]
pub struct Model {
    error: Option<responses::ResponseError>,
    handle: handles::Build,
    info: Option<responses::BuildInfo>,
    nix_log: Option<String>,
}

impl From<Model> for AppUrl {
    fn from(m: Model) -> AppUrl {
        Vec::<String>::from(m.handle).into()
    }
}

#[derive(Clone, Debug)]
pub enum Msg {
    Cancel,
    Error(responses::ResponseError),
    ErrorIgnored,
    Event(Event),
    FetchInfo,
    FetchNixLog,
    GetInfo(responses::BuildInfo),
    GetNixLog(String),
    Noop,
}

pub fn init(orders: &mut impl Orders<Msg>, handle: handles::Build) -> Model {
    orders.send_msg(Msg::FetchInfo);
    Model {
        error: None,
        handle: handle.clone(),
        info: None,
        nix_log: None,
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
            let req = requests::Request::Build(handle, requests::Build::Info);
            perform_request!(
                orders,
                req,
                responses::Response::BuildInfo(info) => Msg::GetInfo(info),
                Msg::Error,
            );
        }
        Msg::FetchNixLog => {
            let handle = model.handle.clone();
            let req = requests::Request::Build(handle, requests::Build::NixLog);
            perform_request!(
                orders,
                req,
                responses::Response::Log(log) => Msg::GetNixLog(log),
                Msg::Error,
            );
        }
        Msg::GetInfo(info) => {
            if info.status == "error" || info.status == "success" {
                orders.send_msg(Msg::FetchNixLog);
            }
            model.info = Some(info);
        }
        Msg::GetNixLog(log) => {
            model.nix_log = Some(log);
        }
        Msg::Noop => (),
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
                p![format!("Output: {}", info.out)],
                if info.dist {
                    let api_url = SETTINGS.get().unwrap().api_server.url(false);
                    a![
                        "Dist",
                        attrs! {
                            At::Href => format!("{}/builds/{}/dist/index.html", api_url, model.handle),
                        },
                    ]
                } else {
                    empty![]
                }
            ],
        },
        match &model.nix_log {
            None => empty![],
            Some(log) => div![h3!["Nix log"], view_log(log.clone()),],
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
