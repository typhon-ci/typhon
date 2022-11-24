use seed::{prelude::*, *};
use typhon_types::*;

#[derive(Clone)]
pub struct Model {
    handle: handles::Project,
    info: Option<responses::ProjectInfo>,
    input_decl: String,
    input_private_key: String,
}

#[derive(Clone)]
pub enum Msg {
    DeclSet,
    Delete,
    Deleted,
    FetchProjectInfo,
    GetProjectInfo(responses::ProjectInfo),
    JobsetsUpdated,
    PrivateKeySet,
    Refresh,
    Refreshed,
    SetDecl,
    SetPrivateKey,
    UpdateInputDecl(String),
    UpdateInputPrivateKey(String),
    UpdateJobsets,
}

pub fn init(orders: &mut impl Orders<Msg>, handle: handles::Project) -> Model {
    orders.send_msg(Msg::FetchProjectInfo);
    Model {
        handle: handle.clone(),
        info: None,
        input_decl: "".into(),
        input_private_key: "".into(),
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::DeclSet => {
            orders.send_msg(Msg::FetchProjectInfo);
        }
        Msg::Delete => {
            let handle = model.handle.clone();
            orders.perform_cmd(async move {
                let req = requests::Request::Project(handle, requests::Project::Delete);
                let rsp = crate::handle_request(&req).await;
                match rsp {
                    Ok(responses::Response::Ok) => Msg::Deleted,
                    _ => todo!(),
                }
            });
        }
        Msg::Deleted => {
            orders.request_url(crate::Urls::home());
        }
        Msg::FetchProjectInfo => {
            let handle = model.handle.clone();
            orders.perform_cmd(async move {
                let req = requests::Request::Project(handle, requests::Project::Info);
                let rsp = crate::handle_request(&req).await;
                match rsp {
                    Ok(responses::Response::ProjectInfo(info)) => Msg::GetProjectInfo(info),
                    _ => todo!(),
                }
            });
        }
        Msg::GetProjectInfo(info) => {
            model.info = Some(info);
        }
        Msg::JobsetsUpdated => {
            orders.send_msg(Msg::FetchProjectInfo);
        }
        Msg::PrivateKeySet => {
            orders.send_msg(Msg::FetchProjectInfo);
        }
        Msg::Refresh => {
            let handle = model.handle.clone();
            orders.perform_cmd(async move {
                let req = requests::Request::Project(handle, requests::Project::Refresh);
                let rsp = crate::handle_request(&req).await;
                match rsp {
                    Ok(responses::Response::Ok) => Msg::Refreshed,
                    _ => todo!(),
                }
            });
        }
        Msg::Refreshed => {
            orders.send_msg(Msg::FetchProjectInfo);
        }
        Msg::SetDecl => {
            let handle = model.handle.clone();
            let decl = model.input_decl.clone();
            model.input_decl = "".into();
            orders.perform_cmd(async move {
                let req = requests::Request::Project(handle, requests::Project::SetDecl(decl));
                let rsp = crate::handle_request(&req).await;
                match rsp {
                    Ok(responses::Response::Ok) => Msg::DeclSet,
                    _ => todo!(),
                }
            });
        }
        Msg::SetPrivateKey => {
            let handle = model.handle.clone();
            let private_key = model.input_decl.clone();
            model.input_private_key = "".into();
            orders.perform_cmd(async move {
                let req = requests::Request::Project(
                    handle,
                    requests::Project::SetPrivateKey(private_key),
                );
                let rsp = crate::handle_request(&req).await;
                match rsp {
                    Ok(responses::Response::Ok) => Msg::PrivateKeySet,
                    _ => todo!(),
                }
            });
        }
        Msg::UpdateInputDecl(decl) => {
            model.input_decl = decl;
        }
        Msg::UpdateInputPrivateKey(private_key) => {
            model.input_private_key = private_key;
        }
        Msg::UpdateJobsets => {
            let handle = model.handle.clone();
            orders.perform_cmd(async move {
                let req = requests::Request::Project(handle, requests::Project::UpdateJobsets);
                let rsp = crate::handle_request(&req).await;
                match rsp {
                    Ok(responses::Response::ProjectUpdateJobsets(_)) => Msg::JobsetsUpdated,
                    _ => todo!(),
                }
            });
        }
    }
}

pub fn view(model: &Model, admin: bool) -> Node<Msg> {
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
                    p![format!("Actions path: {}", info.actions_path)],
                    p![format!("Public key: {}", info.public_key)],
                ],
                div![
                    h3!["Jobsets"],
                    ul![info.jobsets.iter().map(|name| li![a![
                        name,
                        attrs! { At::Href => crate::Urls::jobset(
                            &handles::Jobset {
                                project: model.handle.clone(),
                                jobset: name.into(),
                            }
                        ) },
                    ]])],
                ],
            ],
        },
        if admin {
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
        } else {
            empty![]
        }
    ]
}
