use crate::{perform_request, view_error};
use seed::{prelude::*, *};
use typhon_types::*;

#[derive(Clone)]
pub struct Model {
    error: Option<responses::ResponseError>,
    projects: Vec<String>,
    project_name: String,
}

#[derive(Clone)]
pub enum Msg {
    CreateProject,
    Error(responses::ResponseError),
    ErrorIgnored,
    Event(Event),
    FetchProjects,
    Noop,
    SetProjects(Vec<String>),
    UpdateProjectName(String),
}

pub fn init(orders: &mut impl Orders<Msg>) -> Model {
    orders.send_msg(Msg::FetchProjects);
    Model {
        error: None,
        projects: vec![],
        project_name: "".to_string(),
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::CreateProject => {
            let name = model.project_name.clone();
            model.project_name = "".into();
            let req = requests::Request::CreateProject(handles::project(name));
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
        Msg::UpdateProjectName(name) => {
            model.project_name = name;
        }
    }
}

fn view_home(model: &Model) -> Node<Msg> {
    div![
        h2!["Projects"],
        ul![model.projects.iter().map(|name| li![a![
            name,
            attrs! { At::Href => crate::Urls::project(&handles::project(name.into())) }
        ]])],
    ]
}

fn view_admin(model: &Model) -> Node<Msg> {
    div![
        h2!["Administration"],
        input![
            attrs! {
                At::Value => model.project_name,
            },
            input_ev(Ev::Input, Msg::UpdateProjectName),
        ],
        button!["Create project", ev(Ev::Click, |_| Msg::CreateProject),],
    ]
}

pub fn view(model: &Model, admin: bool) -> Node<Msg> {
    model
        .error
        .as_ref()
        .map(|err| view_error(err, Msg::ErrorIgnored))
        .unwrap_or(div![
            view_home(model),
            if admin { view_admin(model) } else { empty![] },
        ])
}
