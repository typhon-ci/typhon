use crate::pages::login;
use crate::prelude::*;

#[component]
pub fn TyphonLogo() -> impl IntoView {
    let style = style! {
        a {
            display: inline-flex;
            align-items: center;
            padding: 8px;
            user-select: none;
        }
        span {
            position: relative;
            display: inline-block;
            top: -7px;
        }
        div {
            font-variant: small-caps;
            letter-spacing: 0.5px;
            font-size: 28px;
            padding: 4px;
            margin: 2px;
            border-radius: 1px;
            display: inline-block;
            color: var(--color-black);
            position: relative;
            height: 21px;
        }
        a:hover > div:before, a:hover > div:after {
            width: 26%;
        }
        div:before, div:after {
            transition: width 100ms;
            content: raw_str("");
            width: 15%;
            position: absolute;
            top: 0;
            bottom: 0;
            border: 1px solid var(--color-black);
        }
        div:before {
            left: 0;
            border-right: 0px;
        }
        div:after {
            border-left: 0px;
            right: 0;
        }
    };
    view! { class=style,
        <A class=style href="/">
            <div>
                <span>typhon</span>
            </div>
        </A>
    }
}

#[component]
pub fn Nav(route: Signal<Option<routes::Root<routes::Empty>>>) -> impl IntoView {
    let style = style! {
        nav {
            display: inline-flex;
            align-items: center;
        }
        nav :deep(.item) {
            text-decoration: inherit;
            color: inherit;
            font-size: var(--font-size-big);
            padding: "0.25em" "0.5em";
            border-radius: "0.3em";
        }
        nav :deep(.item:hover) {
            background: rgba(208, 215, 222, 0.32);
        }
        nav :deep(.separator) {
            font-size: var(--font-size-small);
            color: var(--color-fg-muted);
            margin: 0;
            padding: 0;
        }
    };
    let contents = move || {
        let Some(route) = route() else {
            return vec![];
        };
        match Option::<handles::Handle>::from(route) {
            None => vec![(Root::Projects, "Projects".into_view())],
            Some(handle) => handle
                .path()
                .map(|handle| {
                    (
                        handle.clone().into(),
                        Vec::<String>::from(handle).last().unwrap().into_view(),
                    )
                })
                .collect(),
        }
        .into_iter()
        .map(move |(href, view)| {
            view! {
                <A class="item" href>
                    {view}
                </A>
            }
            .into_view()
        })
        .intersperse_with(|| view! { <span class="separator">{"/"}</span> }.into_view())
        .collect::<Vec<_>>()
    };
    view! { <nav class=style>{move || contents()}</nav> }
}

#[component]
pub fn Header(#[prop(into)] route: Signal<Option<routes::Root<routes::Empty>>>) -> impl IntoView {
    let style = style! {
        div {
            border-bottom: 1px solid var(--color-border-default);
            display: flex;
            align-items: center;
            background: var(--color-lllightgray);
        }
    };
    let user: Signal<Option<typhon_types::data::User>> = use_context().unwrap();
    view! {
        <div class=style>
            <TyphonLogo/>
            <Nav route/>
            <Transition fallback=move || {
                view! { <span>"Loading..."</span> }
            }>
                {move || {
                    match user() {
                        Some(_) => view! { <login::Logout></login::Logout> }.into_view(),
                        None => view! { <A href="/login">"Log In"</A> }.into_view(),
                    }
                }}

            </Transition>
        </div>
    }
}
