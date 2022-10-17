use yew::prelude::*;
use yew_router::prelude::*;

use gloo_console as console;
use gloo_net::http;
use gloo_utils::format::JsValueSerdeExt;
use serde_json::{Result, Value};
use typhon_types::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, RequestMode, Response}; //::Request;
use yew::{html, Component, Context, Html};

async fn handle_request(request: &requests::Request) -> responses::Response {
    http::Request::new("http://127.0.0.1:8000/api")
        .method(http::Method::POST)
        .json(request)
        .expect("Could not encode request into JSON")
        .send()
        .await
        .unwrap() // TODO: i.e. status != 200, generate
        .json()
        .await
        .unwrap() // TODO: bad JSON
}

#[derive(Debug, Clone, PartialEq)] // Routable
enum Route {
    //#[at("/")]
    Home,
    //#[at("/project/:handle")]
    ProjectHome { handle: handles::Project },
    //#[not_found]
    //#[at("/404")]
    NotFound,
}

// Define the possible messages which can be sent to the component
pub enum Msg {
    SetProjects(Vec<ProjectData>),
    FetchProjects,
}

pub struct Model {
    projects: Vec<ProjectData>,
}

macro_rules! expect {
    ($e:expr, $p:path) => {
        match $e {
            $p(value) => value,
            _ => panic!("expected {}", stringify!($p)),
        }
    };
}

#[derive(Clone, Properties, PartialEq)]
pub struct ProjectData {
    handle: handles::Project,
    infos: responses::ProjectInfo,
}

#[derive(Clone, Properties, PartialEq)]
pub struct ProjectInfoProps {
    pub data: ProjectData,
}

#[function_component(ProjectInfo)]
fn project_info(props: &ProjectInfoProps) -> Html {
    html! {
        <section>
            <header>{props.data.clone().handle}</header>
            <main>
              // <div>{props.data.clone().infos.metadata}</div>
              <div>{props.data.clone().infos.public_key}</div>
              <div>{props.data.clone().infos.decl}</div>
              <div>{props.data.clone().infos.decl_locked}</div>
              <div>{props.data.clone().infos.actions_path}</div>
            </main>
        </section>
    }
    // pub metadata: ProjectMetadata,
    // pub jobsets: Vec<String>,
    // pub public_key: String,
    // pub decl: String,
    // pub decl_locked: String,
    // pub actions_path: String
}

impl Model {
    fn fetch_projects(ctx: &Context<Self>) {
        let link = ctx.link().clone();
        wasm_bindgen_futures::spawn_local(async move {
            let project_handles = expect!(
                handle_request(&requests::Request::ListProjects).await,
                responses::Response::ListProjects
            );

            let projects = futures::future::join_all(project_handles.clone().into_iter().map(
                |project| async {
                    let handle = handles::Project { project };
                    ProjectData {
                        handle: handle.clone(),
                        infos: expect!(
                            handle_request(&requests::Request::Project(
                                handle,
                                requests::Project::Info
                            ))
                            .await,
                            responses::Response::ProjectInfo
                        ),
                    }
                },
            ))
            .await;

            link.send_message(Msg::SetProjects(projects));
        });
    }
}

fn switch(routes: &Route) -> Html {
    match routes {
        Route::Home => html! { <h1>{ "Home" }</h1> },
        _ => html! { <h1>{ "Home" }</h1> },
        // Route::Secure => html! {
        //     <Secure />
        // },
        // Route::NotFound => html! { <h1>{ "404" }</h1> },
    }
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        Self::fetch_projects(ctx);
        Self { projects: vec![] }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SetProjects(l) => {
                self.projects = l;
                true
            }
            Msg::FetchProjects => {
                // todo
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div>
                //<BrowserRouter>
                //    <Switch<Route> render={Switch::render(switch)} />
                //</BrowserRouter>
                <header>
                  <h1>{"typhon"}</h1>
                  <nav>
                  </nav>
                </header>
                <main>
                  <table>
                    <tr>
                      <th>{"Handle"}</th>
                      <th>{"Title"}</th>
                      <th>{"Description"}</th>
                    </tr>
                    {
                        for self.projects.clone().into_iter().map(|project| html!{
                            <tr>
                                <td>{project.handle}</td>
                                <td>{project.infos.clone().metadata.clone().title}</td>
                                <td>{project.infos.clone().metadata.clone().description}</td>
                            </tr>
                        })
                    }
                    </table>
                </main>
            </div>
        }
    }
}
