use crate::server_fn;

use typhon_types::*;

use leptos::*;
use leptos_router::*;

#[component]
pub fn Login(
    action: Action<server_fn::Login, Result<Result<(), responses::ResponseError>, ServerFnError>>,
) -> impl IntoView {
    let value = action.value();
    let has_error =
        move || value.with(|val| matches!(val, Some(Err(_))) || matches!(val, Some(Ok(Err(_)))));
    view! {
        <ActionForm action=action>
            <h2>"Log In"</h2>
            <label>
                "Password:" <input type="password" placeholder="Password" name="password"/>
            </label>
            <br/>
            {move || {
                if has_error() {
                    view! {
                        "Failed to log in!"
                        <br/>
                    }
                        .into_view()
                } else {
                    view! {}.into_view()
                }
            }}

            <button type="submit">"Log In"</button>
        </ActionForm>
    }
}

#[component]
pub fn Logout(action: Action<server_fn::Logout, Result<(), ServerFnError>>) -> impl IntoView {
    view! {
        <ActionForm action=action>
            <button type="submit">"Log Out"</button>
        </ActionForm>
    }
}
