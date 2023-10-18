use crate::perform_request;
use crate::view_error;
use crate::widgets::editable_text;

use seed::{prelude::*, *};

use typhon_types::*;

struct_urls!();

pub struct Model {
    error: Option<responses::ResponseError>,
    handle: handles::Project,
    info: Option<responses::ProjectInfo>,
    declaration_url: editable_text::Model,
    declaration_flake: bool,
    base_url: Url,
}

#[derive(Clone, Debug)]
pub enum Msg {
    //Delete,
    //Deleted,
    Error(responses::ResponseError),
    ErrorIgnored,
    Event(Event),
    FetchInfo,
    GetInfo(responses::ProjectInfo),
    Noop,
    Refresh,
    UpdateJobsets,
    MsgDeclarationUrl(editable_text::Msg),
}

pub fn init(base_url: Url, orders: &mut impl Orders<Msg>, handle: handles::Project) -> Model {
    orders.send_msg(Msg::FetchInfo);

    Model {
        error: None,
        handle: handle.clone(),
        info: None,
        declaration_url: editable_text::init("".to_string()),
        declaration_flake: false,
        base_url,
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    macro_rules! update_text_comp {
        ($m: expr, $t:expr, $mkmsg:expr, |$n:ident| $req:expr) => {
            match editable_text::update($m, $t) {
                Some(editable_text::OutMsg::NewValue($n)) => {
                    let req = requests::Request::Project(
                        model.handle.clone(),
                        $req,
                    );
                    perform_request!(
                        orders,
                        req,
                        responses::Response::Ok => $mkmsg(editable_text::value_synchronized()),
                        Msg::Error,
                    );
                }
                None => (),
            }
        }
    }
    match msg {
        Msg::MsgDeclarationUrl(m) => {
            update_text_comp!(
                m,
                &mut model.declaration_url,
                Msg::MsgDeclarationUrl,
                |url| {
                    requests::Project::SetDecl(requests::ProjectDecl {
                        url,
                        flake: model.declaration_flake,
                    })
                }
            )
        }
        //Msg::Delete => {
        //    let handle = model.handle.clone();
        //    let req = requests::Request::Project(handle, requests::Project::Delete);
        //    perform_request!(
        //        orders,
        //        req,
        //        responses::Response::Ok => Msg::Deleted,
        //        Msg::Error,
        //    );
        //}
        //Msg::Deleted => {
        //    orders.request_url(Urls::home());
        //}
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
            model.info = Some(info.clone());
            model.declaration_url = editable_text::init(info.url.clone());
        }
        Msg::Noop => (),
        Msg::Refresh => {
            model.info = None;
            let handle = model.handle.clone();
            let req = requests::Request::Project(handle, requests::Project::Refresh);
            perform_request!(
                orders,
                req,
                responses::Response::Ok => Msg::Noop,
                Msg::Error,
            );
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

fn show_info_block(
    label: &str,
    class: &str,
    contents: Node<Msg>,
    button: Option<Node<Msg>>,
) -> Node<Msg> {
    div![
        span![label, attrs! { At::Class => "label" }],
        contents,
        button.as_ref().map(|button| vec![button]).unwrap_or(vec![]),
        C![
            class,
            "labeled",
            if button.is_some() {
                "with-button"
            } else {
                "without-button"
            }
        ]
    ]
}

fn view_project(model: &Model, is_admin: bool) -> Node<Msg> {
    let editable_text_view = if is_admin {
        |m: &editable_text::Model, f: Box<dyn FnOnce(String) -> Node<editable_text::Msg>>| {
            editable_text::view(m, f)
        }
    } else {
        |m: &editable_text::Model, f: Box<dyn FnOnce(String) -> Node<editable_text::Msg>>| {
            f(m.text.clone())
        }
    };
    div![
        h2![
            span!["Project "],
            div![
                span![model
                    .info
                    .as_ref()
                    .map(|info| info.metadata.title.clone())
                    .unwrap_or("loading...".into())],
                C!["title", "labeled"]
            ],
            " ",
            div![
                "(id: ",
                code![&model.handle.name, attrs! { At::Class => "id" }],
                ")",
                C!["id"]
            ]
        ],
        button!["Update jobsets", ev(Ev::Click, |_| Msg::UpdateJobsets),],
        match &model.info {
            None => div!["loading..."],
            Some(info) => div![
                div![
                    C!["labels"],
                    show_info_block(
                        "Description",
                        "description",
                        span![&info.metadata.description],
                        None
                    ),
                    show_info_block(
                        "Homepage",
                        "homepage",
                        a![
                            &info.metadata.homepage,
                            attrs! {At::Href => &info.metadata.homepage}
                        ],
                        None
                    )
                ],
                div![
                    C!["labels"],
                    show_info_block(
                        "Flake URI",
                        "desclaration",
                        editable_text_view(&model.declaration_url, Box::new(|s| code![s.clone()]))
                            .map_msg(Msg::MsgDeclarationUrl),
                        Some(div![])
                    ),
                    show_info_block(
                        "Locked flake URI",
                        "locked-declaration",
                        code![if info.url_locked.clone() == "" {
                            "-".into()
                        } else {
                            info.url_locked.clone()
                        }],
                        Some(i![C!["ri-refresh-line"], ev(Ev::Click, |_| Msg::Refresh)])
                    ),
                ],
                div![
                    C!["labels"],
                    show_info_block(
                        "Public key",
                        "desclaration",
                        code![info.public_key.clone()[0..7], "…"],
                        Some({
                            let public_key = info.public_key.clone();
                            i![
                                C!("ri-clipboard-line"),
                                ev(Ev::Click, move |_| copy_to_clipboard(&public_key))
                            ]
                        })
                    ),
                    info.actions_path.as_ref().map(|actions_path| {
                        show_info_block(
                            "Actions path",
                            "actions_path",
                            code![
                                actions_path
                                    .strip_prefix("/nix/store/")
                                    .unwrap_or(actions_path)[0..7],
                                "…"
                            ],
                            Some({
                                let actions_path = actions_path.clone().into();
                                i![
                                    C!("ri-clipboard-line"),
                                    ev(Ev::Click, move |_| copy_to_clipboard(&actions_path))
                                ]
                            }),
                        )
                    })
                ],
                div![
                    h3!["Jobsets"],
                    ul![info.jobsets.iter().map(|name| {
                        let urls = crate::Urls::new(model.base_url.clone());
                        li![a![
                            name,
                            attrs! { At::Href => urls.jobset(
                                &handles::Jobset {
                                    project: model.handle.clone(),
                                    name: name.into(),
                                }
                            ) },
                        ]]
                    })],
                ],
            ],
        },
    ]
}

pub fn view(model: &Model, admin: bool) -> Node<Msg> {
    model
        .error
        .as_ref()
        .map(|err| view_error(&model.base_url, err, Msg::ErrorIgnored))
        .unwrap_or(div![view_project(model, admin),])
}
