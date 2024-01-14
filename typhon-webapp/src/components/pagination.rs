use crate::prelude::*;

use core::ops::Range;
use itertools::Itertools;

#[component]
pub fn Pagination<F>(
    #[prop(into)] range: Range<u32>,
    #[prop(into)] current: Signal<u32>,
    link: F,
) -> impl IntoView
where
    F: Fn(u32) -> String + 'static + Copy,
{
    let style = style! {
        div :deep(> .page) {
            display: inline-block;
            text-decoration: inherit;
            color: inherit;
            padding: 7px 11px;
            margin: 8px 3px;
            border-radius: 5px;
            border: 1px solid transparent;
            user-select: none;
        }
        div :deep(> a.page.active) {
            background: var(--color-bg-emphasis);
            border-color: var(--color-bg-emphasis);
            color: var(--color-fg-emphasis);
        }
        div :deep(a.page:hover) {
            border-color: var(--color-border-default);
        }
        div :deep(.prev-next) {
            display: inline-flex;
            align-items: center;
        }
        div :deep(a.prev-next) {
            color: var(--color-fg-accent);
        }
        div :deep(div.prev-next) {
            color: var(--color-disabled);
        }
    };
    move || {
        let render_button = |page: Option<u32>, contents: View, class: &'static str| {
            move || {
                let active = page == Some(current());
                // let contents = view! {
                //     <div class=format!("page {}", class) class:active=active>
                //         {contents.clone()}
                //     </div>
                // };
                match page {
                    Some(page) => view! {
                        <a class=format!("page {}", class) class:active=active href=link(page)>
                            {contents.clone()}
                        </a>
                    }
                    .into_view(),
                    _ => view! {
                        <div class=format!("page {}", class) class:active=active>
                            {contents.clone()}
                        </div>
                    }
                    .into_view(),
                }
            }
        };
        let around = |i: u32, n: u32| {
            i.checked_sub(n).unwrap_or(u32::MIN).max(range.start)..=(i + n).min(range.end - 1)
        };
        let buttons = around(range.start, 1)
            .chain(around(current(), 2))
            .chain(around(range.end, 1))
            .unique()
            .scan(None, move |prev, n| {
                let diff = prev.map(|prev| n - prev).unwrap_or(1);
                *prev = Some(n);
                let button = render_button(Some(n), n.into_view(), "n");
                let vec = match diff {
                    1 => vec![button],
                    2 => vec![render_button(Some(n - 1), (n - 1).into_view(), "n"), button],
                    _ => vec![render_button(None, "…".into_view(), "sep"), button],
                };
                Some(vec.into_iter())
            })
            .flatten();

        let prev = view! {
            <Icon icon=Icon::from(BiChevronLeftRegular)/>
            "Previous"
        }
        .into_view();
        let next = view! {
            "Next"
            <Icon icon=Icon::from(BiChevronRightRegular)/>
        }
        .into_view();
        let prev = render_button(
            (current() > range.start).then(|| current() - 1),
            prev,
            "prev-next",
        );
        let next = render_button(
            (current() < range.end - 1).then(|| current() + 1),
            next,
            "prev-next",
        );
        use std::iter::once;
        let buttons = once(prev).chain(buttons).chain(once(next));
        view! { <div class=format!("pagination {style}")>{buttons.collect::<Vec<_>>()}</div> }
    }
}
