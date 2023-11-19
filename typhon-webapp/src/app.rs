use typhon_types::*;

use leptos::*;

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
    view! { <p>"Hello world!"</p> }
}
