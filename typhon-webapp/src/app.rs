use crate::home;
use crate::login;
use crate::server_fn;

use typhon_types::*;

use leptos::*;
use leptos_router::*;

#[cfg(feature = "ssr")]
fn event_signal() -> ReadSignal<Option<Event>> {
    let (event, _) = create_signal(None);
    event
}

#[cfg(feature = "hydrate")]
fn event_signal() -> ReadSignal<Option<Event>> {
    use crate::streams::events_stream;
    create_signal_from_stream(events_stream())
}

#[derive(Clone)]
pub struct AllEvents(ReadSignal<Option<Event>>);

impl AllEvents {
    pub fn inner(self) -> ReadSignal<Option<Event>> {
        self.0
    }
}

#[component]
pub fn App() -> impl IntoView {
    let event = event_signal();
    provide_context(AllEvents(event.clone()));

    let login = create_server_action::<server_fn::Login>();
    let logout = create_server_action::<server_fn::Logout>();

    let user = create_resource(
        move || (login.version()(), logout.version()()),
        move |_| server_fn::handle_request(requests::Request::User),
    );
    let user: Signal<Option<data::User>> = Signal::derive(move || match user() {
        Some(Ok(Ok(responses::Response::User(user)))) => user,
        _ => None,
    });
    provide_context(user);

    view! {
        <Router>
            <header>
                <A href="/">
                    <h1>"Typhon"</h1>
                </A>
                <Transition fallback=move || {
                    view! { <span>"Loading..."</span> }
                }>
                    {move || {
                        match user() {
                            Some(_) => {
                                view! { <login::Logout action=logout></login::Logout> }.into_view()
                            }
                            _ => view! { <A href="/login">"Log In"</A> }.into_view(),
                        }
                    }}

                </Transition>
            </header>
            <hr/>
            <main>
                <Routes>
                    <Route ssr=SsrMode::Async path="" view=home::Home/>
                    <Route
                        ssr=SsrMode::Async
                        path="login"
                        view=move || view! { <login::Login action=login></login::Login> }
                    />
                </Routes>
            </main>
        </Router>
    }
}
