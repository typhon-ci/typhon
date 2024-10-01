use crate::prelude::*;

#[component]
fn ProjectList(
    count: Signal<u32>,
    projects: Signal<Vec<(handles::Project, responses::ProjectMetadata)>>,
) -> impl IntoView {
    let style = style! {
        .rows :deep(> .row), .header-columns {
            display: grid;
            grid-template-columns: 1fr 4fr 2fr;
            gap: 10px;
        }
        .header-columns {
            padding-top: 6px;
            color: var(--color-fg-subtle);
            font-size: var(--font-size-small);
        }
        .header {
            display: block!important;
            padding-bottom: 6px;
        }
        .is-table {
            padding-top: 20px;
        }
        .summary {

        }
        .rows :deep(.placeholder) {
            font-style: italic;
            opacity: 0.5;
        }
        .rows :deep(.row.no-project-yet .placeholder) {
            text-align: center;
        }
        .rows :deep(.row.no-project-yet) {
            display: block;
        }
    };
    fn with_placeholder(text: &str) -> impl IntoView {
        if text.is_empty() {
            view! { <span class="placeholder">"<empty>"</span> }
        } else {
            view! { <span>{text.to_string()}</span> }
        }
    }
    view! { class=style,
        <div class="is-table">
            <div class="header">
                <div class="summary">{count} projects</div>
                <div class="header-columns">
                    <div class="column id">"Identifier"</div>
                    <div class="column name">"Name"</div>
                    <div class="column description">"Description"</div>
                </div>
            </div>
            <div class="rows">
                {move || {
                    projects()
                        .is_empty()
                        .then(|| {
                            view! {
                                <div class="row no-project-yet">
                                    <div class="placeholder">No any project yet!</div>
                                </div>
                            }
                        })
                }}
                <For
                    each=projects
                    key=|(handle, _)| handle.clone()
                    children=move |(handle, metadata)| {
                        view! {
                            <div class="row">
                                <div class="column id">
                                    <A href=routes::Root::Project(handle.clone())>{handle.name}</A>
                                </div>
                                <div class="column name">{with_placeholder(&metadata.title)}</div>
                                <div class="column description">
                                    {with_placeholder(&metadata.description)}
                                </div>
                            </div>
                        }
                    }
                />

            </div>
            <Pagination max=10 count current=Signal::derive(|| 1) link=|_: u32| "todo".to_string()/>
        </div>
    }
}

#[component]
pub(crate) fn Projects() -> impl IntoView {
    let user: Signal<Option<data::User>> = use_context().unwrap();

    let (error, count, projects) = {
        let (error, data) = search!(
            Signal::derive(|| 0),
            Signal::derive(|| 100),
            Signal::derive(|| requests::search::Kind::Projects),
            |total, responses::search::Results::Projects(projects)| (total, projects)
        );
        let data = Signal::derive(move || data().unwrap_or((0, Vec::new())));
        (
            error,
            Signal::derive(move || data().0),
            Signal::derive(move || data().1),
        )
    };

    view! {
        <Trans error>
            <ProjectList count projects/>
        </Trans>
    }
}
