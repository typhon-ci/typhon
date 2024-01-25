use crate::prelude::*;

#[component]
pub(crate) fn Projects() -> impl IntoView {
    let (error, projects) = search!(
        Signal::derive(|| 0),
        Signal::derive(|| 100),
        Signal::derive(|| requests::search::Kind::Projects),
        |total, responses::search::Results::Projects(projects)| (total, projects)
    );
    let projects = Signal::derive(move || projects().unwrap_or((0, Vec::new())));
    let count = Signal::derive(move || projects().0);
    let projects = Signal::derive(move || projects().1);
    let action = request_action!(
        CreateProject,
        |name: String, url: String, flake: Option<String>| requests::Request::CreateProject {
            name,
            decl: requests::ProjectDecl {
                url,
                flake: flake.is_some()
            },
        }
    );
    /// TODO split this view in two views, one for the table, one the the form
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
        .row :deep(.empty-field) {
            font-style: italic;
            opacity: 0.5;
        }
    };
    fn with_placeholder(text: &str) -> impl IntoView {
        match text.trim() {
            "" => view! { <span style="opacity: 0.3;">"<empty>"</span> },
            text => view! { <span>text</span> },
        }
    }
    view! { class=style,
        <Trans error>
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
                    <For
                        each=projects
                        key=|(handle, _)| handle.clone()
                        children=move |(handle, metadata)| {
                            view! {
                                <div class="row">
                                    <div class="column id">
                                        <A href=routes::Root::Project(
                                            handle.clone(),
                                        )>{handle.name}</A>
                                    </div>
                                    <div class="column name">
                                        {with_placeholder(&metadata.title)}
                                    </div>
                                    <div class="column description">
                                        {with_placeholder(&metadata.description)}
                                    </div>
                                </div>
                            }
                        }
                    />

                </div>
                <Pagination
                    max=10
                    count
                    current=Signal::derive(|| 1)
                    link=|_: u32| "todo".to_string()
                />

            </div>
        </Trans>
        <ActionForm action>
            <h2>"Add a project"</h2>
            <div>
                <label class="label" for="name">
                    "Identifier"
                </label>
                <input class="input" id="name"/>
            </div>
            <div>
                <label class="label" for="url">
                    "URL"
                </label>
                <input class="input" id="url"/>
            </div>
            <div>
                <label class="label" for="flake">
                    "Flake"
                </label>
                <input class="input" id="flake" type="checkbox" checked=true/>
            </div>
            <button type="submit">
                <Icon icon=Icon::from(BiPlusCircleSolid)/>
                "Add"
            </button>
        </ActionForm>
    }
}
