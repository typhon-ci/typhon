use crate::editable_text;
use crate::{appurl::AppUrl, perform_request, view_error, Urls};

use seed::{prelude::*, *};

use typhon_types::*;

pub struct Model {
    error: Option<responses::ResponseError>,
    handle: handles::Project,
    info: Option<responses::ProjectInfo>,
    declaration: editable_text::Model,
}

impl Model {
    pub fn app_url(&self) -> AppUrl {
        Vec::<String>::from(self.handle.clone()).into()
    }
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
    MsgDeclaration(editable_text::Msg),
}

pub fn init(orders: &mut impl Orders<Msg>, handle: handles::Project) -> Model {
    orders.send_msg(Msg::FetchInfo);

    Model {
        error: None,
        handle: handle.clone(),
        info: None,
        declaration: editable_text::init("".to_string()),
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
        Msg::MsgDeclaration(m) => {
            update_text_comp!(m, &mut model.declaration, Msg::MsgDeclaration, |decl| {
                requests::Project::SetDecl(decl)
            })
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
            model.declaration = editable_text::init(info.decl.clone());
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
                code![&model.handle.project, attrs! { At::Class => "id" }],
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
                        editable_text_view(&model.declaration, Box::new(|s| code![s.clone()]))
                            .map_msg(Msg::MsgDeclaration),
                        Some(div![])
                    ),
                    show_info_block(
                        "Locked flake URI",
                        "locked-declaration",
                        code![if info.decl_locked.clone() == "" {
                            "-".into()
                        } else {
                            info.decl_locked.clone()
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

pub fn view(model: &Model, admin: bool) -> Node<Msg> {
    model
        .error
        .as_ref()
        .map(|err| view_error(err, Msg::ErrorIgnored))
        .unwrap_or(div![view_project(model, admin),])
}
