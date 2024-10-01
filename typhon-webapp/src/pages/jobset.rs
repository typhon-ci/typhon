use crate::prelude::*;

#[component]
pub fn Jobset(
    #[prop(into)] handle: handles::Jobset,
    #[prop(into)] page: Signal<u32>,
) -> impl IntoView {
    let user: Signal<Option<data::User>> = use_context().unwrap();
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
    };
    let (error_info, info) = {
        let handle = handle.clone();
        resource!(
            Signal::derive(move || requests::Request::Jobset(
                handle.clone(),
                requests::Jobset::Info
            )),
            |responses::Response::JobsetInfo(info)| info
        )
    };
    let limit = Signal::derive(move || 10 as u8);
    let offset = Signal::derive(move || (page() - 1) * (limit() as u32));
    let (error_evaluations, evaluations) = {
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
    let signal_handle = {
        let handle = handle.clone();
        Signal::derive(move || handle.clone())
    };
    let item_name = handle.name.clone();
    view! { class=style,
        <Trans error=error_info>
            <PageHeader item_kind="Jobset" item_name=item_name.clone()>
                {move || {
                    info()
                        .map(|info| {
                            view! {
                                <table>
                                    <tr>
                                        <td>"URL"</td>
                                        <td>{info.url}</td>
                                    </tr>
                                    <tr>
                                        <td>"Flake"</td>
                                        <td>{info.flake}</td>
                                    </tr>
                                </table>
                            }
                        })
                }}

            </PageHeader>
        </Trans>
        <Trans error=error_evaluations>
            <Evaluations
                count=evaluation_count
                evaluations
                buttons=Box::new(move || {
                    view! {
                        <Show when=move || user().is_some()>
                            <ActionForm action>
                                <input
                                    type="hidden"
                                    name="project"
                                    value=signal_handle().project.name
                                />
                                <input type="hidden" name="jobset" value=signal_handle().name />
                                <input type="submit" value="Evaluate" />
                            </ActionForm>
                        </Show>
                    }
                        .into_view()
                })
            />

            <Pagination
                max=10
                count=evaluation_count
                current=page
                link={
                    let handle = handle.clone();
                    move |page: u32| String::from(Root::Jobset {
                        handle: handle.clone(),
                        page,
                    })
                }
            />

        </Trans>
    }
}
