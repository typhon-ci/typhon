use crate::prelude::*;

#[component(transparent)]
pub fn Trans<E: Clone + std::fmt::Display + 'static>(
    error: Signal<Option<E>>,
    children: ChildrenFn,
) -> impl IntoView {
    let children = store_value(children);
    view! {
        <Transition>
            <Show
                when=move || { error().is_none() }
                fallback=move || view! { {error().map(|e| format!("Error: {}", e))} }
            >
                {children.with_value(|children| children())}
            </Show>
        </Transition>
    }
}
