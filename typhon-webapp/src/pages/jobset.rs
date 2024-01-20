use crate::prelude::*;

const PAGE_MAX_ITEMS: u8 = 8;

#[component]
pub fn Jobset(
    #[prop(into)] handle: handles::Jobset,
    #[prop(into)] page: Signal<u32>,
) -> impl IntoView {
    let style = style! {
        .pages :deep(a.page) {
            text-decoration: inherit;
            color: inherit;
        }
        .pages :deep(span.page) {
            display: block-inline;
            padding: 8px 12px;
            margin: 8px 3px;
            border-radius: 5px;
        }
        .pages :deep(.active) {
            background: rgb(9, 105, 218);
            color: white;
        }
        header {
            display: block-inline;
            align-items: center;
            padding: 12px;
        }
        header :deep(> span) {
            font-size: var(--font-size-huge);
        }
    };
    let limit = Signal::derive(move || 10 as u8);
    let offset = Signal::derive(move || (page() - 1) * (limit() as u32));
    let (error, evaluations) = {
        let handle = handle.clone();
        search!(
            offset,
            limit,
            Signal::derive({
                let handle = handle.clone();
                move || {
                    requests::search::Kind::Evaluations(requests::search::Evaluation {
                        jobset_name: Some(handle.name.clone()),
                        project_name: Some(handle.project.name.clone()),
                        status: None,
                    })
                }
            }),
            |total, responses::search::Results::Evaluations(evals)| (total, evals)
        )
    };
    let evaluations = Signal::derive(move || evaluations().unwrap_or((0, Vec::new())));
    let evaluation_count = Signal::derive(move || evaluations().0);
    let evaluations = Signal::derive(move || evaluations().1);
    let action = request_action!(EvaluateJobset, |project: String, jobset: String| {
        requests::Request::Jobset(
            handles::Jobset {
                project: handles::Project { name: project },
                name: jobset,
            },
            requests::Jobset::Evaluate(true),
        )
    });
    view! { class=style,
        <header>
            <span>Jobset {handle.name.clone()}</span>

            {
                let handle = handle.clone();
                view! {
                    <ActionForm action>
                        <input type="hidden" name="project" value=handle.project.name/>
                        <input type="hidden" name="jobset" value=handle.name/>
                        <input type="submit" value="Evaluate"/>
                    </ActionForm>
                }
            }

        </header>
        <Trans error>
            <Evaluations count=evaluation_count evaluations/>

            {
                let handle = handle.clone();
                move || {
                    let range = 1..((evaluation_count() as u32).div_ceil(PAGE_MAX_ITEMS as u32)
                        + 1);
                    view! {
                        <Pagination
                            range
                            current=page
                            link={
                                let handle = handle.clone();
                                move |page: u32| String::from(Root::Jobset {
                                    handle: handle.clone(),
                                    page,
                                })
                            }
                        />
                    }
                }
            }

        </Trans>
    }
}
