use seed::{prelude::*, *};
use typhon_types::*;

#[derive(Clone)]
pub struct Model {
    projects: Vec<String>,
    project_name: String,
}

#[derive(Clone)]
pub enum Msg {
    CreateProject,
    Event(Event),
    FetchProjects,
    SetProjects(Vec<String>),
    UpdateProjectName(String),
}

pub fn init(orders: &mut impl Orders<Msg>) -> Model {
    orders.send_msg(Msg::FetchProjects);
    Model {
        projects: vec![],
        project_name: "".to_string(),
    }
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::CreateProject => {
            let name = model.project_name.clone();
            model.project_name = "".into();
            orders.perform_cmd(async move {
                let req = requests::Request::CreateProject(handles::project(name));
                let rsp = crate::handle_request(&req).await;
                match rsp {
                    Ok(responses::Response::Ok) => Msg::FetchProjects,
                    _ => todo!(),
                }
            });
        }
        Msg::Event(_) => {
            orders.send_msg(Msg::FetchProjects);
        }
        Msg::FetchProjects => {
            orders.perform_cmd(async move {
                let req = requests::Request::ListProjects;
                let rsp = crate::handle_request(&req).await;
                match rsp {
                    Ok(responses::Response::ListProjects(l)) => Msg::SetProjects(l),
                    _ => todo!(),
                }
            });
        }
        Msg::SetProjects(l) => {
            model.projects = l;
        }
        Msg::UpdateProjectName(name) => {
            model.project_name = name;
        }
    }
}

pub fn view(model: &Model, admin: bool) -> Node<Msg> {
    div![
        h2!["Projects"],
        ul![model.projects.iter().map(|name| li![a![
            name,
            attrs! { At::Href => crate::Urls::project(&handles::project(name.into())) }
        ]])],
        if admin {
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
        } else {
            empty![]
        }
    ]
}
