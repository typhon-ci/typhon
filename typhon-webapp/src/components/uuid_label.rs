use crate::prelude::*;

#[component]
pub fn UuidLabel(uuid: uuid::Uuid) -> impl IntoView {
    let value = format!("{}", uuid);
    let style = style! {
        input {
            width: 36ch;
            margin: 0;
            padding: 0;
            border: 0;
            font-family: var(--font-family-monospace), monospace;
        }
        input:active, input:focus, input:focus-visible {
            border: 0;
            outline: 0;
        }
    };
    view! { class=style, <input readonly value=value/> }
}
