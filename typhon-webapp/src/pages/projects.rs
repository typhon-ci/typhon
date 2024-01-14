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
    let _total = Signal::derive(move || projects().0);
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
    view! {
        <Trans error>
            <table>
                <tr>
                    <th>"Id"</th>
                    <th>"Name"</th>
                    <th>"Description"</th>
                </tr>
                <For
                    each=projects
                    key=|(handle, _)| handle.clone()
                    children=move |(handle, metadata)| {
                        view! {
                            <tr>
                                <td>
                                    <A href=routes::Root::Project(handle.clone())>{handle.name}</A>
                                </td>
                                <td>{metadata.title}</td>
                                <td>{metadata.description}</td>
                            </tr>
                        }
                    }
                />

            </table>
        </Trans>
        <ActionForm action>
            name: <input type="string" name="name"/> url: <input type="string" name="url"/> flake:
            <input type="checkbox" name="flake"/> <input type="submit"/>
        </ActionForm>
    }
}
