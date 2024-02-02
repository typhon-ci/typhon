use crate::prelude::*;

#[component]
fn Jobset(handle: handles::Jobset) -> impl IntoView {
    let (error, info) = resource!(
        Signal::derive(move || requests::Request::Jobset(handle.clone(), requests::Jobset::Info,)),
        |responses::Response::JobsetInfo(info)| info
    );
    move || {
        view! {
            <Trans error>
                {info()
                    .map(|info| {
                        view! {
                            <div class="row">
                                <div class="column">
                                    <A href=Root::Jobset {
                                        handle: info.handle.clone(),
                                        page: 1,
                                    }>{format!("{}", info.handle.clone().name)}</A>
                                </div>
                            </div>
                        }
                    })}

            </Trans>
        }
    }
}

#[component]
pub(crate) fn Project(handle: handles::Project) -> impl IntoView {
    let user: Signal<Option<data::User>> = use_context().unwrap();
    let (error, info) = {
        let handle = handle.clone();
        resource!(
            Signal::derive(move || requests::Request::Project(
                handle.clone(),
                requests::Project::Info
            )),
            |responses::Response::ProjectInfo(info)| info
        )
    };
    let jobsets = Signal::derive(move || info().map(|x| x.jobsets).unwrap_or(Vec::new()));
    let jobsets = {
        let handle = handle.clone();
        Signal::derive(move || {
            jobsets()
                .into_iter()
                .map(|name| handles::Jobset {
                    project: handle.clone(),
                    name,
                })
                .collect::<Vec<_>>()
        })
    };
    let update_jobsets = request_action!(UpdateJobsets, |name: String| requests::Request::Project(
        handles::Project { name },
        requests::Project::UpdateJobsets,
    ));
    let refresh = request_action!(UpdateJobsets, |name: String| requests::Request::Project(
        handles::Project { name },
        requests::Project::Refresh,
    ));
    let handle_name = {
        let handle_name = handle.name.clone();
        Signal::derive(move || handle_name.clone())
    };
    view! {
        <Trans error>
            <div class="is-table">
                <div class="header">
                    <Show when=move || { user().is_some() }>
                        <ActionForm action=update_jobsets>
                            <input type="hidden" name="name" value=handle_name/>
                            <input type="submit" value="Update jobsets"/>
                        </ActionForm>
                        <ActionForm action=refresh>
                            <input type="hidden" name="name" value=handle_name/>
                            <input type="submit" value="Refresh"/>

                        </ActionForm>
                    </Show>
                    {move || {
                        info()
                            .map(|info| {
                                info.last_refresh
                                    .map(|status| {
                                        let status: data::TaskStatusKind = status.into();
                                        view! { <Status status=move || status.clone()/> }
                                    })
                            })
                            .flatten()
                    }}

                    <div class="header-columns"></div>
                </div>
                <div class="rows">
                    <For
                        each=jobsets
                        key=|handle| handle.name.clone()
                        children=move |handle| {
                            view! { <Jobset handle/> }
                        }
                    />

                </div>

            </div>
            <ul></ul>

        // FIXME: forms need to be in the transition component or else there are hydration bugs
        </Trans>
    }
}
