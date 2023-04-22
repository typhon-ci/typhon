use crate::{perform_request, view_error, Urls};
use seed::{prelude::*, *};
use typhon_types::*;

#[derive(Clone)]
pub struct Model {
    error: Option<responses::ResponseError>,
    handle: handles::Project,
    info: Option<responses::ProjectInfo>,
    input_decl: String,
    input_private_key: String,
}

#[derive(Clone)]
pub enum Msg {
    Delete,
    Deleted,
    Error(responses::ResponseError),
    ErrorIgnored,
    Event(Event),
    FetchInfo,
    GetInfo(responses::ProjectInfo),
    Noop,
    Refresh,
    SetDecl,
    SetPrivateKey,
    UpdateInputDecl(String),
    UpdateInputPrivateKey(String),
    UpdateJobsets,
}

pub fn init(orders: &mut impl Orders<Msg>, handle: handles::Project) -> Model {
    orders.send_msg(Msg::FetchInfo);
    Model {
        error: None,
        handle: handle.clone(),
        info: None,
        input_decl: "".into(),
        input_private_key: "".into(),
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::Delete => {
            let handle = model.handle.clone();
            let req = requests::Request::Project(handle, requests::Project::Delete);
            perform_request!(
                orders,
                req,
                responses::Response::Ok => Msg::Deleted,
                Msg::Error,
            );
        }
        Msg::Deleted => {
            orders.request_url(Urls::home());
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
            let req = requests::Request::Project(handle, requests::Project::Info);
            perform_request!(
                orders,
                req,
                responses::Response::ProjectInfo(info) => Msg::GetInfo(info),
                Msg::Error,
            );
        }
        Msg::GetInfo(info) => {
            model.info = Some(info);
        }
        Msg::Noop => (),
        Msg::Refresh => {
            let handle = model.handle.clone();
            let req = requests::Request::Project(handle, requests::Project::Refresh);
            perform_request!(
                orders,
                req,
                responses::Response::Ok => Msg::Noop,
                Msg::Error,
            );
        }
        Msg::SetDecl => {
            let handle = model.handle.clone();
            let decl = model.input_decl.clone();
            model.input_decl = "".into();
            let req = requests::Request::Project(handle, requests::Project::SetDecl(decl));
            perform_request!(
                orders,
                req,
                responses::Response::Ok => Msg::Noop,
                Msg::Error,
            );
        }
        Msg::SetPrivateKey => {
            let handle = model.handle.clone();
            let private_key = model.input_private_key.clone();
            model.input_private_key = "".into();
            let req =
                requests::Request::Project(handle, requests::Project::SetPrivateKey(private_key));
            perform_request!(
                orders,
                req,
                responses::Response::Ok => Msg::Noop,
                Msg::Error,
            );
        }
        Msg::UpdateInputDecl(decl) => {
            model.input_decl = decl;
        }
        Msg::UpdateInputPrivateKey(private_key) => {
            model.input_private_key = private_key;
        }
        Msg::UpdateJobsets => {
            let handle = model.handle.clone();
            let req = requests::Request::Project(handle, requests::Project::UpdateJobsets);
            perform_request!(
                orders,
                req,
                responses::Response::ProjectUpdateJobsets(_) => Msg::Noop,
                Msg::Error,
            );
        }
    }
}

#[cfg(not(web_sys_unstable_apis))]
fn copy_to_clipboard(_: &String) {
    panic!()
}

#[cfg(web_sys_unstable_apis)]
fn copy_to_clipboard(text: &String) {
    let navigator = seed::window().navigator();
    if let Some(clipboard) = navigator.clipboard() {
        let _ = clipboard.write_text(&text);
    } else {
    }
}

fn view_project(model: &Model) -> Node<Msg> {
    div![
        h2!["Project", " ", &model.handle.project],
        match &model.info {
            None => div!["loading..."],
            Some(info) => div![
                div![
                    p![format!("Title: {}", info.metadata.title)],
                    p![format!("Description: {}", info.metadata.description)],
                    p![
                        "Homepage: ",
                        a![
                            &info.metadata.homepage,
                            attrs! { At::Href => &info.metadata.homepage }
                        ]
                    ],
                ],
                div![
                    h3!["Settings"],
                    p![format!("Declaration: {}", info.decl)],
                    p![format!("Declaration locked: {}", info.decl_locked)],
                    p![format!(
                        "Actions path: {}",
                        info.actions_path.clone().unwrap_or("".into())
                    )],
                    p![format!("Public key: {}", info.public_key)],
                ],
                div![
                    h3!["Jobsets"],
                    ul![info.jobsets.iter().map(|name| li![a![
                        name,
                        attrs! { At::Href => Urls::jobset(
                            &handles::Jobset {
                                project: model.handle.clone(),
                                jobset: name.into(),
                            }
                        ) },
                    ]])],
                ],
            ],
        },
    ]
}

fn view_admin(model: &Model) -> Node<Msg> {
    div![
        h3!["Administration"],
        p![button![
            "Update jobsets",
            ev(Ev::Click, |_| Msg::UpdateJobsets),
        ]],
        p![
            input![
                attrs! {
                    At::Value => model.input_decl,
                },
                input_ev(Ev::Input, Msg::UpdateInputDecl),
            ],
            button!["Set declaration", ev(Ev::Click, |_| Msg::SetDecl),],
            button!["Refresh", ev(Ev::Click, |_| Msg::Refresh),],
        ],
        p![
            input![
                attrs! {
                    At::Value => model.input_private_key,
                },
                input_ev(Ev::Input, Msg::UpdateInputPrivateKey),
            ],
            button!["Set private key", ev(Ev::Click, |_| Msg::SetPrivateKey),]
        ],
        p![button!["Delete", ev(Ev::Click, |_| Msg::Delete),]],
    ]
}

pub fn view(model: &Model, admin: bool) -> Node<Msg> {
    model
        .error
        .as_ref()
        .map(|err| view_error(err, Msg::ErrorIgnored))
        .unwrap_or(div![
            view_project(model),
            if admin { view_admin(model) } else { empty![] },
        ])
}
