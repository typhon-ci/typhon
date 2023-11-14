use crate::resources::*;
use crate::server_fn;

use typhon_types::*;

use leptos::*;
use leptos_router::*;

#[component]
fn CreateProject() -> impl IntoView {
    let action = create_server_action::<server_fn::CreateProject>();
    let value = action.value();
    let has_error =
        move || value.with(|val| matches!(val, Some(Err(_))) || matches!(val, Some(Ok(Err(_)))));
    view! {
        <ActionForm action=action>
            <label>"Name:" <input type="text" name="name"/></label>
            <br/>
            <label>"URL:" <input type="text" name="url"/></label>
            <br/>
            <label>"Flake:" <input type="checkbox" name="flake"/></label>
            <br/>
            {move || {
                if has_error() {
                    view! {
                        "Failed to create a project!"
                        <br/>
                    }
                        .into_view()
                } else {
                    view! {}.into_view()
                }
            }}

            <button type="submit">"Create"</button>
        </ActionForm>
    }
}

#[component]
pub fn Home() -> impl IntoView {
    let user = use_context::<Signal<Option<data::User>>>().unwrap();
    let projects = request(requests::Request::ListProjects);
    let fallback = || view! { <p>"Loading..."</p> };
    view! {
        <Suspense fallback>
            <h2>"Projects:"</h2>
            <ul>
                {projects()
                    .map(|maybe_list| match maybe_list {
                        Ok(Ok(responses::Response::ListProjects(list))) => {
                            list.into_iter()
                                .map(|(name, _)| view! { <li>{name}</li> })
                                .collect_view()
                        }
                        Err(e) => view! { <p>{format!("Error! {}", e)}</p> }.into_view(),
                        _ => view! { <p>"Inconsistant server response"</p> }.into_view(),
                    })}

            </ul>
            {move || {
                match user() {
                    Some(data::User::Admin) => {
                        view! {
                            <h2>"Create project"</h2>
                            <CreateProject/>
                        }
                            .into_view()
                    }
                    _ => view! {}.into_view(),
                }
            }}

        </Suspense>
    }
}
