pub mod evaluations;
pub mod header;
pub mod log;
pub mod pagination;
pub mod status;
pub mod time_related;
pub mod trans;
pub mod uuid_label;

//pub use header::Header;
pub use evaluations::Evaluations;
pub use log::LiveLog;
pub use pagination::Pagination;
pub use status::{HybridStatus, HybridStatusKind, Status};
pub use time_related::*;
pub use trans::Trans;
pub use uuid_label::UuidLabel;

use crate::prelude::*;

#[component]
pub fn PageHeader(
    children: Children,
    #[prop(into)] item_kind: String,
    #[prop(into)] item_name: String,
) -> impl IntoView {
    let style = style! {
        header {
            display: block;
            padding: 12px 18px;
        }
        div.title {
            font-size: var(--font-size-huge);
        }
        .name {
            font-family: var(--font-family-monospace);
        }
        .details {
            color: --color-lgray;
            font-size: 10px;
        }
    };
    view! { class=style,
        <header>
            <div class="title">
                <span class="kind">{item_kind}</span>
                {" "}
                <span class="name">{item_name}</span>
            </div>
            <div class="details">{children()}</div>
        </header>
    }
}

#[component]
pub fn Tag(children: Children, #[prop(into)] href: String) -> impl IntoView {
    let style = style! {
        a {
            display: inline-flex;
            align-items: center;
            padding: 3px;
            margin-left: 3px;
            font-size: var(--font-size-small);
            background: var(--color-bg-accent-muted);
            color: var(--color-fg-accent);
            border-radius: 3px;
            text-decoration: none;
        }
        a:hover {
            text-decoration: underline;
        }
        // .commit, .repo {
        //     display: inline-block;
        // }
        // .commit {
        //     font-size: var(--font-size-big);
        // }
        // .repo {
        // }
        // code {
        //     font-family: var(--font-family-monospace), monospace;
        // }
        // .repo :deep(svg) {
        //     margin-right: 2px;
        // }
    };
    view! { class=style,
        <a href=href class="tag">
            {children()}
        </a>
    }
}
