use crate::prelude::*;

use leptos::*;
use leptos_router::*;

mod server_fn {
    use leptos::*;

    #[server(Login, "/leptos", "Url", "login")]
    pub async fn login(password: String) -> Result<(), ServerFnError> {
        use crate::prelude::*;
        use actix_session::Session;
        use leptos_actix::extract;
        use typhon_types::data::User;
        let res = handle_request!(
            requests::Request::Login { password },
            |responses::Response::Ok| ()
        );
        match res {
            Ok(Ok(())) => {
                let session: Session = extract().await?;
                session.insert("user", User::Admin).map_err(|_| {
                    ServerFnError::<server_fn::error::NoCustomError>::ServerError(
                        "TODO".to_string(),
                    )
                })?;
                //FIXME: a redirect will prevent the deserialization of the output
                //redirect("/");
                Ok(())
            }
            Ok(Err(_)) => Err(ServerFnError::ServerError("TODO".to_string())),
            Err(e) => Err(e),
        }
    }

    #[server(Logout, "/leptos", "Url", "logout")]
    pub async fn logout() -> Result<(), ServerFnError> {
        use actix_session::Session;
        use leptos_actix::extract;
        let session: Session = extract().await?;
        session.remove("user");
        //FIXME: a redirect will prevent the deserialization of the output
        //redirect("/");
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct UserActions {
    login: Action<server_fn::Login, Result<(), ServerFnError>>,
    logout: Action<server_fn::Logout, Result<(), ServerFnError>>,
}

impl UserActions {
    pub fn new() -> Self {
        Self {
            login: create_server_action::<server_fn::Login>(),
            logout: create_server_action::<server_fn::Logout>(),
        }
    }

    pub fn as_signal(self) -> Signal<(usize, usize)> {
        Signal::derive(move || (self.login.version()(), self.logout.version()()))
    }
}

#[component]
pub fn Login() -> impl IntoView {
    let user: Signal<Option<data::User>> = use_context().unwrap();
    let action = use_context::<UserActions>().unwrap().login;
    let value = action.value();
    let has_error = move || value.with(|val| matches!(val, Some(Err(_))));
    view! {
        <Suspense>
            <Show when=move || user().is_none() fallback=|| view! { "You are logged in!" }>
                <ActionForm action>
                    <h2>"Log In"</h2>
                    <div>
                        <label for="password">"Password"</label>
                        <input type="password" placeholder="Password" name="password" />
                    </div>
                    <button type="submit">"Log In"</button>
                    {move || {
                        if has_error() {
                            view! { "Failed to log in!" }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}

                </ActionForm>
            </Show>
        </Suspense>
    }
}

#[component]
pub fn Logout() -> impl IntoView {
    let action = use_context::<UserActions>().unwrap().logout;
    view! {
        <ActionForm action=action>
            <button type="submit">"Log Out"</button>
        </ActionForm>
    }
}
