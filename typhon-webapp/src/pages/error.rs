use crate::prelude::*;

#[component]
pub(crate) fn ErrorPage(
    #[prop(into)] code: String,
    #[prop(into)] message: String,
    children: Children,
) -> impl IntoView {
    let style = style! {
        main {
            padding: 20px;
        }
        #wrapper {
            text-align: center;
        }
    };
    view! { class=style,
        <main>
            <div id="wrapper">
                <h1>{code}</h1>
                <div>{message}</div>
            </div>
            {children()}
        </main>
    }
}

#[component]
pub(crate) fn Unauthorized() -> impl IntoView {
    view! {
        <ErrorPage code="403" message="Sorry, you don't have access to that page.">

            {()}
        </ErrorPage>
    }
}

#[component]
pub(crate) fn BadLocation(loc: leptos_router::Location) -> impl IntoView {
    let style = style! {
        main {
            padding: 20px;
        }
        #wrapper {
            text-align: center;
        }
        details {
            margin-top: 30px;
            font-size: 12px;
            color: var(--color-lgray);
        }
        pre {
            color: black;
            font-size: 11px;
        }
    };
    view! { class=style,
        <ErrorPage code="404" message="The page was not found">

            <details>
                <summary>Details</summary>
                <pre>

                    {
                        #[derive(Debug)]
                        pub struct Location {
                            pub pathname: String,
                            pub search: String,
                            pub query: leptos_router::ParamsMap,
                            pub hash: String,
                            pub state: leptos_router::State,
                        }
                        format!(
                            "{:#?}",
                            Location {
                                pathname: (loc.pathname)(),
                                search: (loc.search)(),
                                query: (loc.query)(),
                                hash: (loc.hash)(),
                                state: (loc.state)(),
                            },
                        )
                    }

                </pre>
            </details>
        </ErrorPage>
    }
}
