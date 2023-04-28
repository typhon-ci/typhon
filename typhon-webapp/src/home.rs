use crate::{appurl::AppUrl, perform_request, view_error};
use seed::{prelude::*, *};
use typhon_types::*;

#[derive(Clone, Default)]
pub struct Model {
    error: Option<responses::ResponseError>,
    projects: Vec<(String, responses::ProjectMetadata)>,
    new_project: (String, String),
}
impl From<Model> for AppUrl {
    fn from(_: Model) -> AppUrl {
        AppUrl::default()
    }
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
    UpdateNewProjectExpr(String),
}

pub fn init(orders: &mut impl Orders<Msg>) -> Model {
    orders.send_msg(Msg::FetchProjects);
    Model::default()
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::CreateProject => {
            let req = requests::Request::CreateProject {
                handle: handles::project(model.new_project.0.clone()),
                decl: model.new_project.1.clone(),
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
        Msg::UpdateNewProjectExpr(expr) => {
            model.new_project.1 = expr;
        }
    }
}

fn view_home(model: &Model, admin: bool) -> Node<Msg> {
    div![
        h2!["Projects"],
        table![
            tr![th!["Id"], th!["Name"], th!["Description"],],
            model.projects.iter().map(|(name, meta)| tr![
                td![a![
                    name,
                    attrs! { At::Href => crate::Urls::project(&handles::project(name.into())) }
                ]],
                td![String::from(meta.title.clone())],
                td![String::from(meta.description.clone())],
            ])
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
                    label!["Flake URI:"],
                    input![
                        attrs! {
                            At::Value => model.new_project.1,
                            At::Placeholder => "github:org/repo",
                        },
                        input_ev(Ev::Input, Msg::UpdateNewProjectExpr),
                        enter
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
        .map(|err| view_error(err, Msg::ErrorIgnored))
        .unwrap_or_else(|| view_home(model, admin))
}
