use crate::perform_request;
use crate::view_error;
use seed::{prelude::*, *};
use typhon_types::*;

struct_urls!();

#[derive(Clone, Default)]
pub struct Model {
    error: Option<responses::ResponseError>,
    projects: Vec<(String, responses::ProjectMetadata)>,
    new_project: (String, String, bool),
    base_url: Url,
}

#[derive(Debug, Clone)]
pub enum Msg {
    CreateProject,
    Error(responses::ResponseError),
    ErrorIgnored,
    Event(Event),
    FetchProjects,
    Noop,
    SetProjects(Vec<(String, responses::ProjectMetadata)>),
    UpdateNewProjectName(String),
    UpdateNewProjectUrl(String),
    UpdateNewProjectFlake,
}

pub fn init(base_url: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders.send_msg(Msg::FetchProjects);
    Model {
        error: None,
        projects: Vec::new(),
        new_project: ("".to_string(), "".to_string(), false),
        base_url,
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::CreateProject => {
            let req = requests::Request::CreateProject {
                name: model.new_project.0.clone(),
                decl: requests::ProjectDecl {
                    url: model.new_project.1.clone(),
                    flake: model.new_project.2.clone(),
                },
            };
            model.new_project = <_>::default();
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
            orders.send_msg(Msg::FetchProjects);
        }
        Msg::FetchProjects => {
            let req = requests::Request::ListProjects;
            perform_request!(
                orders,
                req,
                responses::Response::ListProjects(l) => Msg::SetProjects(l),
                Msg::Error,
            );
        }
        Msg::Noop => (),
        Msg::SetProjects(l) => {
            model.projects = l;
        }
        Msg::UpdateNewProjectName(name) => {
            model.new_project.0 = name;
        }
        Msg::UpdateNewProjectUrl(url) => {
            model.new_project.1 = url;
        }
        Msg::UpdateNewProjectFlake => {
            model.new_project.2 = !model.new_project.2;
        }
    }
}

fn view_home(model: &Model, admin: bool) -> Node<Msg> {
    div![
        h2!["Projects"],
        table![
            tr![th!["Id"], th!["Name"], th!["Description"],],
            model.projects.iter().map(|(name, meta)| {
                let urls = crate::Urls::new(&model.base_url);
                tr![
                    td![a![
                        name,
                        attrs! { At::Href => urls.project(&handles::project(name.into())) }
                    ]],
                    td![String::from(meta.title.clone())],
                    td![String::from(meta.description.clone())],
                ]
            })
        ],
        admin.then(|| {
            let empty = model.new_project == <_>::default();
            let enter = (!empty).then(|| {
                keyboard_ev(Ev::KeyUp, |e| {
                    if e.key() == "Enter" {
                        Some(Msg::CreateProject)
                    } else {
                        None
                    }
                })
            });
            section![
                h2!["Add a project"],
                div![
                    label!["Identifier:"],
                    input![
                        attrs! {
                            At::Value => model.new_project.0,
                            At::Placeholder => "myproject",
                        },
                        input_ev(Ev::Input, Msg::UpdateNewProjectName),
                        enter.clone()
                    ],
                    label!["Flake URL:"],
                    input![
                        attrs! {
                            At::Value => model.new_project.1,
                            At::Placeholder => "github:org/repo",
                        },
                        input_ev(Ev::Input, Msg::UpdateNewProjectUrl),
                        enter
                    ],
                    label!["Flake:"],
                    input![
                        attrs! {
                            At::Value => model.new_project.2,
                            At::Type => "checkbox",
                        },
                        input_ev(Ev::Change, |_| Msg::UpdateNewProjectFlake),
                    ],
                    div![],
                    button![
                        "Add project",
                        (!empty).then(|| ev(Ev::Click, |_| Msg::CreateProject)),
                        empty.then(|| attrs! {At::Disabled => true}),
                    ],
                    C!["add-project"],
                ],
            ]
        })
    ]
}

pub fn view(model: &Model, admin: bool) -> Node<Msg> {
    model
        .error
        .as_ref()
        .map(|err| view_error(&model.base_url, err, Msg::ErrorIgnored))
        .unwrap_or_else(|| view_home(model, admin))
}
