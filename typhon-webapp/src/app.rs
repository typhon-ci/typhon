use crate::prelude::*;

use typhon_types::*;

use leptos::*;
use leptos_meta::*;
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
    provide_meta_context();

    let event = event_signal();
    provide_context(AllEvents(event.clone()));

    let user_actions = pages::login::UserActions::new();
    let user = create_resource(user_actions.as_signal(), move |_| async {
        use responses as res;
        handle_request!(requests::Request::User, |res::Response::User(user)| user)
    });
    let user: Signal<Option<data::User>> = Signal::derive(move || match user() {
        Some(Ok(Ok(user))) => user,
        _ => None,
    });
    provide_context(user);
    provide_context(user_actions);

    let _styler_class = style! {
        :deep(body) {
            font-family: Roboto;
            font-weight: 300;
            margin: 0;
            padding: 0;
        }
        :deep(:root) {
            --font-size-huge: 20px;
            --font-size-big: 16px;
            --font-size-normal: 14px;
            --font-size-small: 12px;
            font-size: var(--font-size-normal);
            --font-family-monospace: "JetBrains Mono";

            --color-white: white;
            --color-blue: #0969da;
            --color-lightblue: #ddf4ff;
            --color-black: #1f2328;
            --color-ligthblack: #24292f;
            --color-gray: #656d76;
            --color-lgray: #6e7781;
            --color-llgray: #8c959f;
            --color-lightgray: #d0d7de;
            --color-lllightgray: #f4f5f7; /* left pane selected button background */
            --color-llightgray: #d8dee4;
            --color-red: #cf222e;
            --color-lightred: #d1242f;
            --color-green: #1a7f37;
            --color-lightgreen: #1f883d;
            --color-orange: rgb(219, 171, 10);

            --color-fg-emphasis: var(--color-white);
            --color-fg-accent: var(--color-blue);
            --color-bg-accent-muted: var(--color-lightblue);
            --color-bg-emphasis: var(--color-blue);
            --color-fg: var(--color-black);
            --color-border-default: var(--color-lightgray);
            --color-border-muted: var(--color-llightgray);
            --color-fg-muted: var(--color-gray);
            --color-bg-muted: transparent;
            --color-fg-subtle: var(--color-lgray);
            --color-fg-btn: var(--color-lightblack);
            --color-danger: var(--color-lightred);
            --color-danger-emphasis: var(--color-red);
            --color-success: var(--color-green);
            --color-green-button-bg: var(--color-lightgreen);
            --color-bg-light: var(--color-lllightgray);
            --color-disabled: var(--color-llgray);

            --color-task-status-success: var(--color-success);
            --color-task-status-error: var(--color-danger);
            --color-task-status-canceled: var(--color-fg-muted);
            --color-task-status-pending: var(--color-orange);

            --status-font-size: var(--font-size-huge);
        }
        :deep(*[data-status=Success]) {
            --color-task-status: var(--color-task-status-success);
        }
        :deep(*[data-status=Error]) {
            --color-task-status: var(--color-task-status-error);
        }
        :deep(*[data-status=Canceled]) {
            --color-task-status: var(--color-task-status-canceled);
        }
        :deep(*[data-status=Pending]) {
            --color-task-status: var(--color-task-status-pending);
        }

        :deep(.is-table > .header) {
            --radius: 8px;
        }
        :deep(.is-table > .header, .is-table > .rows > .row) {
            margin: 0px 20px;
            display: flex;
            align-items: center;
            border: 1px solid var(--color-border-default);
            padding: 12px;
        }
        :deep(.is-table > .rows > .row) {
            border-top: 0px;
        }
        :deep(.is-table > .header) {
            border-radius: var(--radius) var(--radius) 0 0;
            background: var(--color-bg-light);
        }
        :deep(.is-table .rows > .row:last-child) {
            border-radius: 0 0 var(--radius) var(--radius);
        }
    };
    provide_context(utils::now_signal());
    view! { class=_styler_class,
        <Router>
            <Style>{include_str!("../../target/main.css")}</Style>
            <Stylesheet href="/assets/node_modules/@fontsource/jetbrains-mono/index.css"/>
            <Stylesheet href="/assets/node_modules/@fontsource/roboto/index.css"/>
            <Routes>
                <Route path="/*any" view=routes::Router ssr=SsrMode::Async/>
            </Routes>
        </Router>
    }
}

#[component]
fn Project(name: String) -> impl IntoView {
    let info = create_resource(
        {
            let name = name.clone();
            move || name.clone()
        },
        |name: String| async move {
            use requests as req;
            use responses as res;
            let project = handles::Project { name };
            handle_request!(
                req::Request::Project(project.clone(), req::Project::Info),
                |res::Response::ProjectInfo(info)| info
            )
            .unwrap()
        },
    );
    view! {
        <Suspense>
            <b>{format!("{:#?}", info.get())}</b>
        </Suspense>
    }
    // #[derive(PartialEq, Clone, Params)]
    // struct ProjectParams {
    //     id: String,
    // }
    // let params = use_params::<ProjectParams>().get().unwrap();
    // params.id
}

// /// This is the page of an evaluation: project is defined, jobset as well.
// #[component]
// fn Test() -> impl IntoView {
//     // use handles::*;
//     // let project = Project { name: "hi".into() };
//     // let jobset = Jobset {
//     //     project,
//     //     name: "main".into(),
//     // };
//     // let handle = Evaluation { jobset, num: 10 };
//     // let (handle, _) = create_signal(handle);
//     // view! { <Evaluation handle/> }
// }
